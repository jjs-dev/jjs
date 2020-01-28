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
}
pub fn task_run(opts: Opts) -> anyhow::Result<()> {
    if opts.build {
        println!("Building");
        let mut cmd = Command::new("cargo");
        cmd.arg("jjs-build").arg("--docker");
        if opts.debug {
            cmd.arg("--configure-opt=--docker-build-opt=--progress=plain");
        }
        cmd.try_exec()?;
    }
    println!("dropping existing docker-compose");
    Command::new("docker-compose")
        .arg("down")
        .arg("-v")
        .try_exec()?;
    println!("starting jjs");
    Command::new("docker-compose")
        .arg("up")
        .arg("--detach")
        .try_exec()?;
    if opts.test {
        println!("Waiting for start");
        Command::new("cargo")
            .arg("run")
            .arg("--package")
            .arg("util")
            .env("RUST_LOG", "debug")
            .env("JJS_WAIT", "tcp://localhost:1779")
            .try_exec()?;
        println!("Executing tests");
        Command::new("cargo")
            .arg("jjs-test")
            .arg("--integration-tests")
            .arg("--skip-unit")
            .try_exec()?;
    }
    Ok(())
}
