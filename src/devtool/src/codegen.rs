use anyhow::Context as _;
use std::{path::Path, process::Command};
use util::cmd::CommandExt as _;

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

pub(crate) fn task_codegen() -> anyhow::Result<()> {
    println!("Obtaining schemas");
    let api_schema = read_openapi().context("get models")?;
    let out_path = "src/apiserver/openapi-gen.json";
    let schema = serde_json::to_string_pretty(&api_schema)?;
    std::fs::write(out_path, schema)?;
    println!("Building generator");
    if std::env::var("JJS_CODEGEN_DOCKER_NO_BUILD").is_err() {
        let mut cmd = make_docker();
        cmd.arg("build");
        cmd.arg("-f").arg("./src/devtool/openapigen.Dockerfile");
        cmd.arg("./src/devtool/scripts"); //just some random dir
        cmd.arg("-t").arg("jjs-openapi-generator");
        cmd.try_exec().context("failed to build docker image")?;
    }
    println!("Running client codegen");
    let mut gen = make_docker();
    gen.arg("run");
    gen.arg("--interactive").arg("--rm");

    gen.arg("--mount").arg(format!(
        "type=bind,source={},target=/input",
        Path::new("./src/apiserver").canonicalize()?.display()
    ));
    gen.arg("--mount").arg(format!(
        "type=bind,source={},target=/output",
        Path::new("./src/gen-api-client").canonicalize()?.display()
    ));
    gen.arg("jjs-openapi-generator");
    gen.arg("generate");
    gen.arg("--input-spec").arg("/input/openapi-gen.json");
    gen.arg("--output").arg("/output");
    gen.arg("--generator-name").arg("rust");
    gen.try_exec()?;
    println!("Formatting");
    Command::new("cargo")
        .arg("fmt")
        .current_dir("src/gen-api-client")
        .try_exec()?;

    let main_file_path = "src/gen-api-client/src/lib.rs";
    let old_content = std::fs::read_to_string(main_file_path)?;
    let new_content = format!(
        "#![allow(dead_code)]\n#![allow(clippy::all)]\n{}",
        old_content
    );
    std::fs::write(main_file_path, new_content)?;
    std::fs::remove_file("src/gen-api-client/git_push.sh")?;
    Ok(())
}
