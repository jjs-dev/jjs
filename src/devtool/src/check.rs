use anyhow::Context as _;
use clap::Clap;
use log::{debug, info};
use std::process::Command;
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


#[derive(Clap)]
pub struct CheckOpts {
    /// Run autopep8
    #[clap(long)]
    autopep8: bool,
    /// Run shellcheck
    #[clap(long)]
    shellcheck: bool,
    /// Do not run default checks
    #[clap(long)]
    no_default: bool,
    /// Exit with status 1 as soon as any invoked command fails
    #[clap(long)]
    pub(crate) fail_fast: bool,
}

pub fn check(opts: &CheckOpts, runner: &Runner) -> anyhow::Result<()> {
    if opts.autopep8 || !opts.no_default {
        autopep8(runner);
    }
    if opts.shellcheck || !opts.no_default {
        shellcheck(runner);
    }
    Ok(())
}
