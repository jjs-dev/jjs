mod check;
mod runner;

use crate::runner::Runner;
use std::{env::set_current_dir, path::Path, process::Command};
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
    /// Perform build & install
    #[structopt(name = "build")]
    Build,
    /// remove target files, related to JJS. This should prevent cache invalidation
    #[structopt(name = "ci-clean")]
    CiClean,
}

trait CommandExt {
    fn run_on(&mut self, runner: &Runner);

    fn cargo_color(&mut self);
}

impl CommandExt for Command {
    fn run_on(&mut self, runner: &Runner) {
        runner.exec(self);
    }

    fn cargo_color(&mut self) {
        if atty::is(atty::Stream::Stdout) {
            self.args(&["--color", "always"]);
            self.env("RUST_LOG_STYLE", "always");
        }
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

fn get_package_list() -> Vec<String> {
    let t = std::fs::read_to_string("Cargo.toml").unwrap();
    let t: toml::Value = toml::from_str(&t).unwrap();
    let t = t.get("workspace").unwrap().get("members").unwrap();
    let t = t.as_array().unwrap();
    t.iter()
        .map(|val| val.as_str().unwrap().to_string())
        .collect()
}

fn task_ci_clean() {
    let mut pkgs = get_package_list();
    pkgs.push("rand-ffi".to_string());
    for s in &mut pkgs {
        s.push('-');
    }
    let process_dir = |path: &Path| {
        for item in std::fs::read_dir(path).unwrap() {
            let item = item.unwrap();
            let name = item.file_name();
            let name = name.to_str().unwrap();
            let is_from_jjs = pkgs
                .iter()
                .any(|pkg| name.starts_with(pkg) && !name.contains("cfg-if"));
            if is_from_jjs {
                let p = item.path();
                println!("removing: {}", p.display());
                if p.is_file() {
                    std::fs::remove_file(p).unwrap();
                } else {
                    std::fs::remove_dir_all(p).unwrap();
                }
            }
        }
    };
    process_dir(Path::new("target/debug/deps"));
    process_dir(Path::new("target/debug/.fingerprint"));
    process_dir(Path::new("target/debug/build"));
    process_dir(Path::new("target/debug/incremental"));
}

fn task_build(runner: &Runner) {
    std::fs::File::create("./target/.jjsbuild").unwrap();
    Command::new("../configure")
        .current_dir("target")
        .args(&["--prefix", "/opt/jjs"])
        .args(&["--disable-core", "--disable-tools", "--disable-testlib"])
        .run_on(runner);

    Command::new("make").current_dir("target").run_on(runner);
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
        CliArgs::CiClean => task_ci_clean(),
        CliArgs::Build => task_build(&runner),
    }
    runner.exit_if_errors();
    eprintln!("OK");
}

fn ci() -> bool {
    std::env::var("TRAVIS_RUST_VERSION").is_ok()
}
