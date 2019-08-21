mod check;
mod runner;

use crate::runner::Runner;
use std::{env::set_current_dir, process::Command};
use structopt::StructOpt;

#[derive(StructOpt)]
struct TestArgs {
    #[structopt(long = "verbose")]
    verbose: bool,
}

#[derive(StructOpt)]
enum CliArgs {
    /// Lint project
    #[structopt(name = "check")]
    Check(check::CheckOpts),
    /// Run all tests
    #[structopt(name = "test")]
    Test(TestArgs),
    /// Clean all build files except Cargo's
    #[structopt(name = "clean")]
    Clean,
}

trait CommandExt {
    fn run_on(&mut self, runner: &Runner);
}

impl CommandExt for Command {
    fn run_on(&mut self, runner: &Runner) {
        runner.exec(self);
    }
}

fn task_test(args: TestArgs, runner: &Runner) {
    let mut cmd = Command::new("cargo");
    cmd.args(&["test"]);
    if args.verbose {
        cmd.args(&["--", "--nocapture"]);
    }
    cmd.run_on(runner);
}

fn task_clean() {
    use std::fs::{remove_dir_all, remove_file};
    remove_dir_all("./target/jtl-cpp").ok();
    remove_dir_all("./target/deb").ok();
    remove_file("./target/minion-ffi-prepend.h").ok();
    remove_file("./target/minion-ffi.h").ok();
    remove_file("./target/Makefile").ok();
    remove_file("./target/make").ok();
    remove_file("./target/jjs-build-config.json").ok();

    remove_dir_all("./minion-ffi/example-c/cmake-build").ok();
    remove_dir_all("./minion-ffi/example-c/cmake-build-debug").ok();
    remove_dir_all("./minion-ffi/example-c/cmake-build-release").ok();

    remove_dir_all("./jtl-cpp/cmake-build").ok();
    remove_dir_all("./jtl-cpp/cmake-build-debug").ok();
    remove_dir_all("./jtl-cpp/cmake-build-release").ok();
}

fn main() {
    env_logger::init();
    set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();
    let args = CliArgs::from_args();
    let mut runner = Runner::new();
    match args {
        CliArgs::Check(opts) => {
            runner.set_fail_fast(opts.fail_fast);
            check::check(&opts, &runner)
        }
        CliArgs::Test(args) => task_test(args, &runner),
        CliArgs::Clean => task_clean(),
    }
    runner.exit_if_errors();
}

fn ci() -> bool {
    std::env::var("TRAVIS_RUST_VERSION").is_ok()
}
