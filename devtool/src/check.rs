use super::CommandExt;
use crate::runner::Runner;
use log::{debug, info};
use std::{path::PathBuf, process::Command};
use structopt::StructOpt;

fn cmake_bin() -> &'static str {
    if crate::ci() {
        "/opt/cmake/bin/cmake"
    } else {
        "cmake"
    }
}

fn rustfmt(runner: &Runner) {
    info!("running cargo fmt --check");
    Command::new("cargo")
        .args(&["fmt", "--verbose", "--all", "--", "--check"])
        .run_on(runner);
}

fn clippy(runner: &Runner) {
    info!("running clippy");
    Command::new("cargo")
        .args(&[
            "clippy",
            "--all",
            "--tests",
            "--",
            "-D",
            "clippy::all",
            "-D",
            "warnings",
        ])
        .run_on(runner);
}

fn find_scripts() -> impl Iterator<Item = PathBuf> {
    let mut types_builder = ignore::types::TypesBuilder::new();
    types_builder.add_defaults();
    types_builder.negate("all");
    types_builder.select("sh");
    let types_matched = types_builder.build().unwrap();
    ignore::WalkBuilder::new(".")
        .types(types_matched)
        .build()
        .map(Result::unwrap)
        .filter(|x| {
            let ty = x.file_type();
            match ty {
                Some(f) => f.is_file(),
                None => false,
            }
        })
        .map(|x| x.path().to_path_buf())
}

fn shellcheck(runner: &Runner) {
    const SCRIPTS_CHECK_BATCH_SIZE: usize = 10;
    info!("checking scripts");
    let scripts = find_scripts().collect::<Vec<_>>();
    for script_chunk in scripts.chunks(SCRIPTS_CHECK_BATCH_SIZE) {
        let mut cmd = Command::new("shellcheck");
        cmd.arg("--color=always");
        // FIXME: cmd.arg("--check-sourced");
        // requires using fresh shellcheck on CI
        for scr in script_chunk {
            debug!("checking script {}", scr.display());
            cmd.arg(scr);
        }
        cmd.run_on(runner);
    }
}

fn build_minion_ffi_example(runner: &Runner) {
    info!("building minion-ffi C example");
    std::fs::create_dir("minion-ffi/example-c/cmake-build-debug").ok();
    Command::new(cmake_bin())
        .current_dir("./minion-ffi/example-c/cmake-build-debug")
        .arg("..")
        .run_on(runner);
    Command::new(cmake_bin())
        .current_dir("./minion-ffi/example-c/cmake-build-debug")
        .arg("--build")
        .arg(".")
        .run_on(runner);
}

fn check_testlib(opts: &CheckOpts, runner: &Runner) {
    info!("checking testlib");
    std::fs::create_dir("jtl-cpp/cmake-build-debug").ok();
    Command::new(cmake_bin())
        .current_dir("./jtl-cpp/cmake-build-debug")
        .arg("-DCMAKE_EXPORT_COMPILE_COMMANDS=On")
        .arg("..")
        .run_on(runner);
    Command::new(cmake_bin())
        .current_dir("./jtl-cpp/cmake-build-debug")
        .args(&["--build", "."])
        .args(&["--target", "all"])
        .run_on(runner);
    if opts.pvs {
        Command::new("pvs-studio-analyzer")
            .current_dir("./jtl-cpp/cmake-build-debug")
            .arg("analyze")
            .run_on(runner);

        let report_path = "./jtl-cpp/cmake-build-debug/pvs";
        let diagnostics = "GA:1,2;64:1,2;OP:1,2,3";

        std::fs::remove_dir_all(report_path).ok();

        let output_type = if crate::ci() { "errorfile" } else { "fullhtml" };

        Command::new("plog-converter")
            .current_dir("./jtl-cpp/cmake-build-debug")
            .args(&["-a", diagnostics])
            .args(&["-t", output_type])
            .arg("PVS-Studio.log")
            .args(&["-o", "pvs"])
            .run_on(runner);
    }
}

#[derive(StructOpt)]
pub struct CheckOpts {
    /// Do not run clippy
    #[structopt(long = "no-clippy")]
    no_clippy: bool,
    /// Do not run rustfmt
    #[structopt(long = "no-rustfmt")]
    no_rustfmt: bool,
    /// Do not run shellcheck
    #[structopt(long = "no-shellcheck")]
    no_shellcheck: bool,
    /// Do not build minion-ffi C example
    #[structopt(long = "no-minion-ffi-c-example")]
    no_minion_ffi_example: bool,
    /// Do not build testlib
    #[structopt(long = "no-testlib")]
    no_testlib: bool,
    /// Use PVS-Studio to analyze testlib
    #[structopt(long = "pvs")]
    pvs: bool,
    /// Exit with status 1 as soon as any invoked command fails
    #[structopt(long = "fail-fast")]
    pub(crate) fail_fast: bool,
}

pub fn check(opts: &CheckOpts, runner: &Runner) {
    if !opts.no_rustfmt {
        rustfmt(runner);
    }
    if !opts.no_shellcheck {
        shellcheck(runner);
    }
    if !opts.no_minion_ffi_example {
        build_minion_ffi_example(runner);
    }
    if !opts.no_testlib {
        check_testlib(opts, runner);
    }
    if !opts.no_clippy {
        clippy(runner);
    }
}
