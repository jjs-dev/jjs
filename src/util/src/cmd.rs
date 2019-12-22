use std::{
    process::{exit, Command},
    sync::atomic::{AtomicBool, Ordering},
};
use anyhow::{Context, bail};

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
        if self.had_errors.load(Ordering::SeqCst) {
            eprintln!("Action was not successful: some commands failed");
            exit(1);
        }
    }

    pub fn error(&self) {
        if self.fail_fast {
            eprintln!("Exiting because fail-fast mode enabled");
            exit(1);
        } else {
            self.had_errors.store(true, Ordering::SeqCst);
            log::debug!("Error reported");
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
        let st = self
            .status()
            .with_context(|| format!("failed to start {:?}", self))?;
        if st.success() {
            Ok(())
        } else {
            bail!("child command {:?} failed", self)
        }
    }
}
