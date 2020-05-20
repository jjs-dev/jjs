use log::{debug, error, info};
use std::process::Command;
use structopt::StructOpt;
use util::cmd::{CommandExt, Runner};

fn cmake_bin() -> &'static str {
    "cmake"
}

fn autopep8(runner: &Runner) {
    info!("running autopep8");
    let python_files: Vec<_> =
        crate::glob_util::find_items(crate::glob_util::ItemKind::Python).collect();

    let mut cmd = Command::new("autopep8");
    cmd.arg("--exit-code");
    cmd.arg("--diff");
    for file in python_files {
        cmd.arg(file);
    }
    cmd.run_on(runner);
}

fn shellcheck(runner: &Runner) {
    const SCRIPTS_CHECK_BATCH_SIZE: usize = 10;
    info!("checking scripts");
    let scripts: Vec<_> = crate::glob_util::find_items(crate::glob_util::ItemKind::Bash).collect();
    for script_chunk in scripts.chunks(SCRIPTS_CHECK_BATCH_SIZE) {
        let mut cmd = Command::new("shellcheck");
        cmd.arg("--color=always");
        // TODO: cmd.arg("--check-sourced");
        // requires using fresh shellcheck on CI
        for scr in script_chunk {
            debug!("checking script {}", scr.display());
            cmd.arg(scr);
        }
        cmd.run_on(runner);
    }
}

pub(crate) fn build_minion_ffi_tests(runner: &Runner) {
    info!("building minion-ffi tests");
    std::fs::create_dir("src/minion-ffi/tests/cmake-build-debug").ok();
    Command::new(cmake_bin())
        .current_dir("./src/minion-ffi/tests/cmake-build-debug")
        .arg("..")
        .run_on(runner);
    Command::new(cmake_bin())
        .current_dir("./src/minion-ffi/tests/cmake-build-debug")
        .arg("--build")
        .arg(".")
        .run_on(runner);
}

fn pvs(runner: &Runner) {
    Command::new("pvs-studio-analyzer")
        .current_dir("./jtl-cpp/cmake-build-debug")
        .arg("analyze")
        .args(&["--exclude-path", "./jtl-cpp/deps"])
        .args(&["-j", "4"])
        .run_on(runner);

    let diagnostics_important = "GA:1,2;64:1,2;OP:1,2,3";
    let diagnostics_additional = "GA:3;64:3";

    let output_type = "errorfile";

    let do_convert = |diag_spec: &str, name: &str| {
        let report_path = format!("./jtl-cpp/cmake-build-debug/pvs-{}", name);
        std::fs::remove_dir_all(&report_path).ok();

        Command::new("plog-converter")
            .current_dir("./jtl-cpp/cmake-build-debug")
            .args(&["--analyzer", diag_spec])
            .args(&["--renderTypes", output_type])
            .arg("PVS-Studio.log")
            .args(&["--output", &format!("pvs-{}", name)])
            .run_on(runner);
        println!("---info: PVS report {}---", name);
        let report_text = std::fs::read_to_string(&report_path)
            .unwrap_or_else(|err| format!("failed to read report: {}", err));
        // skip first line which is reference to help
        let report_text = report_text
            .splitn(2, '\n')
            .nth(1)
            .map(str::to_string)
            .unwrap_or_default();
        println!("{}\n---", report_text);
        !report_text.chars().any(|c| !c.is_whitespace())
    };

    if !do_convert(diagnostics_important, "high") {
        error!("PVS found some errors");
        runner.error();
    }
    do_convert(diagnostics_additional, "low");
}

fn check_testlib(runner: &Runner) {
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
}

#[derive(StructOpt)]
pub struct CheckOpts {
    /// Run autopep8
    #[structopt(long)]
    autopep8: bool,
    /// Run shellcheck
    #[structopt(long)]
    shellcheck: bool,
    /// Build minion-ffi tests
    #[structopt(long)]
    minion_ffi: bool,
    /// Build testlib
    #[structopt(long)]
    testlib: bool,
    /// Use PVS-Studio to analyze testlib
    #[structopt(long)]
    pvs: bool,
    /// Do not run default checks
    #[structopt(long)]
    no_default: bool,
    /// Exit with status 1 as soon as any invoked command fails
    #[structopt(long)]
    pub(crate) fail_fast: bool,
}

fn secrets_enabled() -> bool {
    let val = match std::env::var("SECRET_ENABLED") {
        Ok(val) => val,
        Err(_) => return false, // definitely not a CI
    };
    !val.trim().is_empty()
}

pub fn check(opts: &CheckOpts, runner: &Runner) {
    if opts.autopep8 || !opts.no_default {
        autopep8(runner);
    }
    if opts.shellcheck || !opts.no_default {
        shellcheck(runner);
    }
    if opts.minion_ffi || !opts.no_default {
        build_minion_ffi_tests(runner);
    }
    if opts.testlib || !opts.no_default {
        check_testlib(runner);
    }
    let force_pvs = std::env::var("CI").is_ok() && secrets_enabled();
    let force_not_pvs = std::env::var("CI").is_ok() && !secrets_enabled();
    if (opts.pvs || force_pvs) && !force_not_pvs {
        pvs(runner);
    }
}
