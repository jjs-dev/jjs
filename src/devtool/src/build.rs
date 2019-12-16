use std::process::Command;
use structopt::StructOpt;
use util::cmd::{CommandExt, Runner};

#[derive(StructOpt)]
pub(crate) struct RawBuildOpts {
    /// enable things that are not required for running tests
    #[structopt(long)]
    full: bool,
    /// Enable docker
    #[structopt(long)]
    docker: bool,
    /// Setup (useful for development)
    #[structopt(long)]
    setup: bool,
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

    fn should_build_docker(&self) -> bool {
        self.0.docker || crate::ci::detect_build_type().is_ci()
    }

    fn raw(&self) -> &RawBuildOpts {
        &self.0
    }
}

pub(crate) fn task_build(opts: RawBuildOpts, runner: &Runner) {
    let opts = BuildOpts(opts);
    std::fs::File::create("./target/.jjsbuild").unwrap();
    let mut cmd = Command::new("../configure");
    cmd.current_dir("target");
    cmd.arg("--out=/opt/jjs");

    if opts.full() {
        // TODO: enable when deb support is OK
        // cmd.arg("--enable-deb");
    }
    // useful for easily starting up & shutting down
    // required for docker compose
    if opts.should_build_docker() {
        cmd.arg("--enable-docker");
    }
    if opts.full() {
        cmd.arg("--enable-archive");
        cmd.arg("--enable-extras");
    }
    if !opts.should_build_man() {
        cmd.arg("--disable-man");
    }
    if !cmd.run_on(runner) {
        return;
    }

    Command::new("make").current_dir("target").run_on(runner);

    runner.exit_if_errors();

    if opts.raw().setup {
        println!("running setup");
        Command::new("/opt/jjs/bin/jjs-setup")
            .arg("--data-dir=/tmp/jjs")
            .arg("--install-dir=/opt/jjs")
            .arg("--db-url=postgres://jjs:internal@localhost:5432/jjs")
            .arg("--force")
            .arg("--sample-contest")
            .arg("--symlink-config")
            .arg("--setup-config")
            .arg("--toolchains")
            .run_on(runner);
    }
}
