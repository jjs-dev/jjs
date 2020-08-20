//! Core valuing logic
//! It is extracted to library to simplify testing

#[cfg(test)]
mod tests;

pub mod cfg;
mod fiber;

pub use cfg::Config;

use anyhow::{Context, Result};
use fiber::{Fiber, FiberReply};
use invoker_api::valuer_proto::{JudgeLogKind, ProblemInfo, TestDoneNotification, ValuerResponse};
use log::debug;
use pom::TestId;
use std::collections::HashSet;
/// SValuer is pure. Only `ValuerDriver` actually performs some IO, interacting with environment, such as JJS invoker.
pub trait ValuerDriver: std::fmt::Debug {
    /// Retrieves `ProblemInfo`. Will be called once.
    fn problem_info(&mut self) -> Result<ProblemInfo>;
    /// Sends valuer response
    fn send_command(&mut self, cmd: &ValuerResponse) -> Result<()>;
    /// Polls notification about test finish
    fn poll_notification(&mut self) -> Result<Option<TestDoneNotification>>;
}

/// SValuer itself
#[derive(Debug)]
pub struct SimpleValuer<'a> {
    driver: &'a mut dyn ValuerDriver,
    /// Amount of tests that are currently running.
    running_tests: u32,
    /// How many fibers did not emit judge log yet
    running_fibers: usize,
    /// Amount of tests that were requested to run.
    /// It is used for caching purposes.
    used_tests: HashSet<TestId>,
    fibers: Vec<Fiber>,
}

impl<'a> SimpleValuer<'a> {
    pub fn new(
        driver: &'a mut dyn ValuerDriver,
        cfg: &'a cfg::Config,
    ) -> anyhow::Result<SimpleValuer<'a>> {
        let problem_info = driver
            .problem_info()
            .context("failed to query problem info")?;
        let mut fibers = Vec::new();

        fibers.push(Fiber::new(cfg, &problem_info, JudgeLogKind::Full));
        fibers.push(Fiber::new(cfg, &problem_info, JudgeLogKind::Contestant));

        let fibers_cnt = fibers.len();
        Ok(SimpleValuer {
            driver,
            running_tests: 0,
            used_tests: HashSet::new(),
            fibers,
            running_fibers: fibers_cnt,
        })
    }

    /// Creates ValuerResponse for executing test `test_id`.
    /// Returns early if this test was already requested.
    fn send_run_on_test_query(&mut self, test_id: TestId, live: bool) -> anyhow::Result<()> {
        if !self.used_tests.insert(test_id) {
            return Ok(());
        }
        let cmd = ValuerResponse::Test { test_id, live };
        self.running_tests += 1;

        self.driver
            .send_command(&cmd)
            .context("failed to send TEST command")?;
        Ok(())
    }

    /// Executes one iteration.
    /// Returns false when valuing finishes.
    fn step(&mut self) -> anyhow::Result<bool> {
        debug!("Running next step");

        debug!("Polling fibers");
        // do we have something new from fibers?
        for fiber in &mut self.fibers {
            let reply = fiber.poll();
            debug!("Polling fiber {:?}: {:?}", fiber.kind(), &reply);
            match reply {
                FiberReply::LiveScore { score } => {
                    if fiber.kind() == JudgeLogKind::Contestant {
                        debug!("Step done: sending live score");
                        let live_score = ValuerResponse::LiveScore { score };
                        self.driver
                            .send_command(&live_score)
                            .context("failed to send new live score")?;
                        return Ok(true);
                    } else {
                        debug!("Ignoring live score: kind mismatch");
                    }
                }
                FiberReply::Test { test_id } => {
                    let is_live = self.fibers.iter().any(|fib| fib.test_is_live(test_id));
                    debug!(
                        "Step done: test execution requested (test id {}, live: {})",
                        test_id, is_live
                    );
                    self.send_run_on_test_query(test_id, is_live)?;
                    return Ok(true);
                }
                FiberReply::Finish(judge_log) => {
                    debug!("Step done: new judge log {:?} emitted", judge_log.kind);
                    let resp = ValuerResponse::JudgeLog(judge_log);
                    self.running_fibers -= 1;
                    self.driver
                        .send_command(&resp)
                        .context("failed to submit judge log")?;
                    return Ok(true);
                }
                FiberReply::None => {
                    debug!("No updates from this fiber");
                    continue;
                }
            }
        }
        // do we have pending notifications?
        if let Some(notification) = self
            .driver
            .poll_notification()
            .context("failed to poll for notification")?
        {
            debug!("Step done: got notification");
            self.process_notification(notification);
            return Ok(true);
        }

        // do we have running tests?
        if self.running_tests != 0 {
            debug!("Step done: waiting for running tests completion");
            return Ok(true);
        }
        if self.running_fibers != 0 {
            debug!("Step done: waiting for running fibers completion");
            return Ok(true);
        }

        Ok(false)
    }

    /// Runs to valuing completion
    pub fn exec(mut self) -> anyhow::Result<()> {
        loop {
            let should_run = self.step()?;
            if !should_run {
                break;
            }
        }
        self.driver.send_command(&ValuerResponse::Finish)
    }

    fn process_notification(&mut self, notification: TestDoneNotification) {
        assert_ne!(self.running_tests, 0);
        self.running_tests -= 1;
        for fiber in self.fibers.iter_mut() {
            fiber.add(&notification);
        }
    }
}

pub mod status_util {
    pub fn make_ok_status() -> invoker_api::Status {
        invoker_api::Status {
            code: "OK".to_string(),
            kind: invoker_api::StatusKind::Accepted,
        }
    }

    pub fn make_err_status() -> invoker_api::Status {
        invoker_api::Status {
            code: "NOT_OK".to_string(),
            kind: invoker_api::StatusKind::Rejected,
        }
    }
}
