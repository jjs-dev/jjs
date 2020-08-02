use anyhow::Context as _;
use log::{debug, info};
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

fn static_analysis() -> anyhow::Result<()> {
    std::fs::remove_dir_all("src/jtl/cmake-build-analysis").ok();
    std::fs::create_dir_all("src/jtl/cmake-build-analysis")?;
    Command::new("scan-build")
        .arg(cmake_bin())
        .current_dir("./src/jtl/cmake-build-analysis")
        .arg("..")
        .try_exec()?;

    let analysis_output_dir = tempfile::TempDir::new().context("failed to get temp dir")?;

    Command::new("scan-build")
        .current_dir("./src/jtl/cmake-build-analysis")
        .arg("-o")
        .arg(&analysis_output_dir.path())
        .arg("make")
        .args(&["-j", "4"])
        .try_exec()?;

    let dir_items = std::fs::read_dir(analysis_output_dir.path())?.count();
    if dir_items != 0 {
        // make sure dir is saved
        analysis_output_dir.into_path();
        anyhow::bail!("Analyzer found bugs");
    }

    Ok(())
}

fn check_testlib(runner: &Runner) {
    info!("checking testlib");
    std::fs::create_dir("src/jtl/cmake-build-debug").ok();
    Command::new(cmake_bin())
        .current_dir("./src/jtl/cmake-build-debug")
        .arg("-DCMAKE_EXPORT_COMPILE_COMMANDS=On")
        .arg("..")
        .run_on(runner);
    Command::new(cmake_bin())
        .current_dir("./src/jtl/cmake-build-debug")
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
    /// Build testlib
    #[structopt(long)]
    testlib: bool,
    /// Analyze testlib
    #[structopt(long)]
    clang_analyzer: bool,
    /// Do not run default checks
    #[structopt(long)]
    no_default: bool,
    /// Exit with status 1 as soon as any invoked command fails
    #[structopt(long)]
    pub(crate) fail_fast: bool,
}

pub fn check(opts: &CheckOpts, runner: &Runner) -> anyhow::Result<()> {
    if opts.autopep8 || !opts.no_default {
        autopep8(runner);
    }
    if opts.shellcheck || !opts.no_default {
        shellcheck(runner);
    }
    if opts.testlib || !opts.no_default {
        check_testlib(runner);
    }
    if opts.clang_analyzer {
        static_analysis().context("static analysis failed")?;
    }
    Ok(())
}
