mod build;
mod check;
mod tests;

use std::{env::set_current_dir, path::Path};
use structopt::StructOpt;
use tests::{task_test, TestArgs};
use util::cmd::Runner;

#[derive(StructOpt)]
enum CliArgs {
    /// Lint project
    Check(check::CheckOpts),
    /// Run all tests
    Test(TestArgs),
    /// Clean all build files except Cargo's
    Clean,
    /// Perform build & install
    Build(build::RawBuildOpts),
    /// remove target files, related to JJS. This should prevent cache invalidation
    CiClean,
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
    let is_from_jjs = |name: &str| {
        if name.contains("cfg-if") || name.contains("cfg_if") {
            return false;
        }
        pkgs.iter().any(|pkg| {
            let pname = name.replace('_', "-");
            let libpkg = format!("lib{}", &pkg);
            pname.starts_with(pkg) || pname.starts_with(&libpkg)
        })
    };
    let process_dir = |path: &Path| {
        for item in std::fs::read_dir(path).unwrap() {
            let item = item.unwrap();
            let name = item.file_name();
            let name = name.to_str().unwrap();
            if is_from_jjs(name) {
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

fn main() {
    env_logger::init();
    set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).unwrap();
    let args = CliArgs::from_args();
    let mut runner = Runner::new();
    match args {
        CliArgs::Check(opts) => {
            runner.set_fail_fast(opts.fail_fast);
            check::check(&opts, &runner)
        }
        CliArgs::Test(args) => {
            runner.set_fail_fast(args.fail_fast);
            task_test(args, &runner)
        }
        CliArgs::Clean => task_clean(),
        CliArgs::CiClean => task_ci_clean(),
        CliArgs::Build(opts) => build::task_build(opts, &runner),
    }
    runner.exit_if_errors();
    eprintln!("OK");
}
