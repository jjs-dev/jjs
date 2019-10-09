use std::process::Command;
use structopt::StructOpt;
use util::cmd::{CommandExt, Runner};

#[derive(StructOpt)]
pub(crate) struct RawBuildOpts {
    /// enable things that are not required for running tests
    full: bool,
}

struct BuildOpts(RawBuildOpts);

impl BuildOpts {
    fn full(&self) -> bool {
        let bt = util::ci::detect_build_type();
        bt.is_deploy() || self.0.full
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
    if !cmd.run_on(runner) {
        return;
    }

    Command::new("make").current_dir("target").run_on(runner);
}
