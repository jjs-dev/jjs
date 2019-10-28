use std::process::Command;
use structopt::StructOpt;
use util::cmd::{CommandExt, Runner};

#[derive(StructOpt)]
pub(crate) struct RawBuildOpts {
    /// enable things that are not required for running tests
    #[structopt(long)]
    full: bool,
}

struct BuildOpts(RawBuildOpts);

impl BuildOpts {
    fn full(&self) -> bool {
        let bt = crate::ci::detect_build_type();
        bt.is_deploy() || self.0.full
    }

    fn should_build_man(&self) -> bool {
        let bt = crate::ci::detect_build_type();
        bt.deploy_info().contains(&crate::ci::DeployKind::Man) || bt.is_not_ci()
    }
}

pub(crate) fn task_build(opts: RawBuildOpts, runner: &Runner) {
    let opts = BuildOpts(opts);
    std::fs::File::create("./target/.jjsbuild").unwrap();
    let mut cmd = Command::new("../configure");
    cmd.current_dir("target");
    cmd.args(&["--out", "/opt/jjs"]);

    if opts.full() {
        cmd.arg("--enable-deb");
    }
    // useful for easily starting up & shutting down
    // required for docker compose
    cmd.args(&["--enable-docker", "--docker-tag", "jjs-%:dev"]);
    if opts.full() {
        cmd.arg("--enable-archive");
    }
    if !opts.should_build_man() {
        cmd.arg("--disable-man");
    }
    if !cmd.run_on(runner) {
        return;
    }

    Command::new("make").current_dir("target").run_on(runner);
}
