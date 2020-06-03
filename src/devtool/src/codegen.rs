use anyhow::Context as _;
use std::{path::Path, process::Command};
use util::cmd::CommandExt as _;

#[derive(structopt::StructOpt)]
pub(crate) struct Opts {
    /// Print debugging information (currently schema converted to v2)
    #[structopt(long)]
    debug: bool,
}

fn read_openapi() -> anyhow::Result<serde_json::Value> {
    let mut cmd = Command::new("python");
    cmd.arg("./main.py");
    cmd.current_dir("./src/apiserver");
    cmd.env("__JJS_SPEC", "openapi");
    let out = cmd.try_exec_with_output().context("exec apiserver")?;
    let data = serde_json::from_slice(&out.stdout)?;
    Ok(data)
}

fn make_docker() -> Command {
    let mut probe = Command::new("podman");
    probe.arg("--help");
    if probe.try_exec_with_output().is_ok() {
        Command::new("podman")
    } else {
        Command::new("docker")
    }
}

const IMAGE_NAME: &str = "docker.pkg.github.com/jjs-dev/openapi-gen/gen:latest";

pub(crate) fn task_codegen(opts: Opts) -> anyhow::Result<()> {
    println!("Obtaining schemas");
    let api_schema = read_openapi().context("get models")?;
    let out_path = "src/apiserver/openapi.json";
    let schema = serde_json::to_string_pretty(&api_schema)?;
    std::fs::write(out_path, schema)?;

    println!("Pulling generator");
    {
        let mut cmd = make_docker();
        cmd.arg("pull");
        cmd.arg(IMAGE_NAME);
        cmd.try_exec()?;
    }
    println!("Running client codegen");
    let mut gen = make_docker();
    gen.arg("run");
    gen.arg("--interactive").arg("--rm");

    gen.arg("--mount").arg(format!(
        "type=bind,source={},target=/in",
        Path::new("./src/apiserver").canonicalize()?.display()
    ));
    gen.arg("--mount").arg(format!(
        "type=bind,source={},target=/out",
        Path::new("./src/gen-api-client").canonicalize()?.display()
    ));
    if opts.debug {
        gen.arg("--env").arg("PRINT_CONVERTED_SCHEMA=1");
    }
    gen.arg(IMAGE_NAME);
    gen.try_exec()?;
    {
        let manifest_path = "src/gen-api-client/Cargo.toml";
        let old_content = std::fs::read_to_string(manifest_path)?;
        let new_content = old_content.replace("[workspace]", "");
        std::fs::write(manifest_path, new_content)?;
    }
    {
        let main_path = "src/gen-api-client/lib.rs";
        let old_content = std::fs::read_to_string(main_path)?;
        let new_content = format!("{}{}", ALLOW, old_content);
        std::fs::write(main_path, new_content)?;
    }
    println!("Formatting");
    Command::new("cargo")
        .arg("fmt")
        .current_dir("src/gen-api-client")
        .try_exec()?;
    Ok(())
}

const ALLOW: &str = r#"
#![allow(clippy::borrow_interior_mutable_const)]
#![allow(clippy::identity_conversion)]
#![allow(clippy::wrong_self_convention)]
"#;
