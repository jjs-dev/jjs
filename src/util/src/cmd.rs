use log::error;
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
        if self.had_errors.load(Ordering::SeqCst) {
            eprintln!("Action was not successful: some commands returned non-zero");
            exit(1);
        }
    }

    pub fn error(&self) {
        if self.fail_fast {
            eprintln!("Exiting because fail-fast mode enabled");
            exit(1);
        } else {
            self.had_errors.store(true, Ordering::SeqCst);
        }
    }

    pub fn exec(&self, cmd: &mut Command) -> bool {
        let st = cmd.status().unwrap();
        if st.success() {
            true
        } else {
            error!("child command failed");
            self.error();
            false
        }
    }
}

pub trait CommandExt {
    fn run_on(&mut self, runner: &Runner) -> bool;

    fn cargo_color(&mut self);
}

impl CommandExt for Command {
    fn run_on(&mut self, runner: &Runner) -> bool {
        runner.exec(self)
    }

    fn cargo_color(&mut self) {
        if atty::is(atty::Stream::Stdout) {
            self.args(&["--color", "always"]);
            self.env("RUST_LOG_STYLE", "always");
        }
    }
}
