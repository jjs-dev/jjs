use log::error;
use std::{
    process::{exit, Command},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Runner {
    fail_fast: bool,
    had_errors: AtomicBool,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            fail_fast: false,
            had_errors: AtomicBool::new(false),
        }
    }

    pub fn set_fail_fast(&mut self, ff: bool) {
        self.fail_fast = ff;
    }
}

impl Runner {
    pub fn exit_if_errors(&self) {
        if self.had_errors.load(Ordering::SeqCst) {
            eprintln!("Check was not successful: some commands returned non-zero");
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

    pub fn exec(&self, cmd: &mut Command) {
        let st = cmd.status().unwrap();
        if !st.success() {
            error!("child command failed");
            self.error();
        }
    }
}
