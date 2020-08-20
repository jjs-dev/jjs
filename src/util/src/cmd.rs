use anyhow::{bail, Context};
use std::{
    process::{exit, Command},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
pub struct Runner {
    fail_fast: bool,
    had_errors: AtomicBool,
}

impl Runner {
    pub fn new() -> Self {
        Runner::default()
    }

    pub fn set_fail_fast(&mut self, ff: bool) {
        self.fail_fast = ff;
    }
}

impl Runner {
    pub fn exit_if_errors(&self) {
        if self.has_error() {
            eprintln!("Action was not successful: some commands failed");
            exit(1);
        }
    }

    pub fn has_error(&self) -> bool {
        self.had_errors.load(Ordering::SeqCst)
    }

    pub fn error(&self) {
        if self.fail_fast {
            eprintln!("Exiting because fail-fast mode enabled");
            exit(1);
        } else {
            self.had_errors.store(true, Ordering::SeqCst);
            tracing::debug!("Error reported");
        }
    }

    pub fn exec(&self, cmd: &mut Command) {
        let is_err = cmd.try_exec().is_err();
        if is_err {
            self.error();
        }
    }
}

pub trait CommandExt {
    fn run_on(&mut self, runner: &Runner);

    fn try_exec(&mut self) -> Result<(), anyhow::Error>;
    fn try_exec_with_output(&mut self) -> Result<std::process::Output, anyhow::Error>;

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

    fn try_exec(&mut self) -> anyhow::Result<()> {
        let status = self
            .status()
            .with_context(|| format!("failed to start {:?}", self))?;
        if status.success() {
            Ok(())
        } else {
            bail!("child command {:?} failed", self)
        }
    }

    fn try_exec_with_output(&mut self) -> anyhow::Result<std::process::Output> {
        let output = self
            .output()
            .with_context(|| format!("failed to start {:?}", self))?;
        if output.status.success() {
            Ok(output)
        } else {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("{}", String::from_utf8_lossy(&output.stderr));
            bail!("child command {:?} failed", self)
        }
    }
}
