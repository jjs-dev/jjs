use std::process::Command;
use structopt::StructOpt;
use util::cmd::CommandExt;
#[derive(StructOpt)]
pub struct Opts {
    #[structopt(long, short = "b")]
    build: bool,
    #[structopt(long, short = "t")]
    test: bool,
    #[structopt(long)]
    debug: bool,
    #[structopt(long)]
    nocapture: bool,
    #[structopt(long)]
    podman: bool
}
pub fn task_run(opts: Opts) -> anyhow::Result<()> {
    let compose_bin = if opts.podman {
        "podman-compose"
    } else {
        "docker-compose"
    };
    println!("dropping existing docker-compose");
    Command::new(compose_bin).arg("down").try_exec()?;
    if opts.build {
        println!("Building");
        let mut cmd = Command::new("cargo");
        cmd.arg("jjs-build").arg("--docker");
        if opts.debug {
            cmd.arg("--configure-opt=--docker-build-opt=--progress=plain");
        }
        cmd.try_exec()?;
    }
    println!("starting jjs");
    Command::new(compose_bin)
        .arg("up")
        .arg("--detach")
        .try_exec()?;
    if opts.test {
        println!("Waiting for start");
        Command::new("cargo")
            .arg("run")
            .arg("--package")
            .arg("util")
            .env("RUST_LOG", "info,util=debug")
            .env("JJS_WAIT", "http://localhost:1779/")
            .try_exec()?;
        println!("Executing tests");
        let mut cmd = Command::new("cargo");
        cmd.arg("jjs-test")
            .arg("--integration-tests")
            .arg("--skip-unit");
        if opts.nocapture {
            cmd.arg("--nocapture");
        }
        cmd.try_exec()?;
    }
    Ok(())
}
