//! Core valuing logic
//! It is extracted to library to simplify testing
use anyhow::{Context, Result};
use invoker_api::valuer_proto::{self, ProblemInfo, TestDoneNotification, ValuerResponse};
use pom::TestId;
use std::collections::{HashMap, HashSet, VecDeque};
/// SValuer is pure. Only `ValuerDriver` actually performs some IO, interacting with environment, such as JJS invoker.
pub trait ValuerDriver {
    /// Retrieves `ProblemInfo`. Will be called once.
    fn problem_info(&mut self) -> Result<ProblemInfo>;
    /// Sends valuer response
    fn send_command(&mut self, cmd: &ValuerResponse) -> Result<()>;
    /// Polls notification about test finish
    fn poll_notification(&mut self) -> Result<Option<TestDoneNotification>>;
}

/// SValuer itself
pub struct SimpleValuer<'a> {
    driver: &'a mut dyn ValuerDriver,
    test_storage: TestStorage,
    score: u32,
}

impl SimpleValuer<'_> {
    pub fn new(driver: &mut dyn ValuerDriver) -> anyhow::Result<SimpleValuer> {
        let problem_info = driver
            .problem_info()
            .context("failed to query problem info")?;
        let test_storage = TestStorage::new(problem_info.test_count);
        Ok(SimpleValuer {
            driver,
            test_storage,
            score: 0,
        })
    }

    /// Runs to valuing completion
    pub fn exec(mut self) -> anyhow::Result<()> {
        loop {
            // do we have pending notifications ?
            if let Some(notification) = self
                .driver
                .poll_notification()
                .context("failed to poll for notification")?
            {
                self.process_notification(notification);
                continue;
            }
            // do we have tests to run ?
            if let Some(tid) = self.test_storage.poll_test() {
                let resp = ValuerResponse::Test {
                    test_id: tid,
                    live: true,
                };
                self.driver
                    .send_command(&resp)
                    .context("failed to send TEST command")?;
                continue;
            }
            break;
        }
        self.driver.send_command(&ValuerResponse::Finish {
            score: self.score,
            treat_as_full: false,
            judge_log: valuer_proto::JudgeLog {
                tests: vec![],
                subtasks: vec![],
                name: "todo".to_string(),
            },
        })
    }

    fn process_notification(&mut self, notification: TestDoneNotification) {
        if notification.test_status.kind.is_success() {
            self.score += 1;
            self.test_storage.mark_ok(notification.test_id)
        }
    }
}

/// Utility struct which works with tests, groups, dependencies etc
struct TestStorage {
    tests: HashSet<TestId>,
    deps: HashMap<TestId, HashSet<TestId>>,
    deps_rev: HashMap<TestId, Vec<TestId>>,
    queue: VecDeque<TestId>,
}

impl TestStorage {
    /// Initializes some fields to meaningful values
    fn init(&mut self) {
        // calc deps_rev
        for &v in &self.tests {
            if !self.deps.contains_key(&v) {
                continue;
            }
            for &w in &self.deps[&v] {
                self.deps_rev.entry(w).or_default().push(v);
            }
        }
        // calc queue
        for &v in &self.tests {
            if !self.deps.contains_key(&v) || self.deps[&v].is_empty() {
                self.queue.push_back(v);
            }
        }
    }

    fn new(cnt: u32) -> Self {
        let mut ts = TestStorage {
            tests: HashSet::new(),
            deps: HashMap::new(),
            deps_rev: HashMap::new(),
            queue: VecDeque::new(),
        };
        for test_id in 1..=cnt {
            ts.tests.insert(TestId::make(test_id));
        }
        for test_id in 2..=cnt {
            ts.deps
                .entry(TestId::make(test_id))
                .or_default()
                .insert(TestId::make(test_id - 1));
        }
        ts.init();
        ts
    }

    fn mark_ok(&mut self, test: TestId) {
        if !self.deps_rev.contains_key(&test) {
            return;
        }
        let dependants = self.deps_rev[&test].iter().copied();
        let deps = &mut self.deps;
        for dependant in dependants {
            deps.get_mut(&dependant).unwrap().remove(&test);
            if deps[&dependant].is_empty() {
                self.queue.push_back(dependant);
            }
        }
    }

    fn poll_test(&mut self) -> Option<TestId> {
        self.queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod test_storage {
        use super::*;
        #[test]
        fn simple() {
            let mut ts = TestStorage::new(3);
            assert_eq!(ts.poll_test(), Some(TestId::make(1)));
            assert_eq!(ts.poll_test(), None);
            ts.mark_ok(TestId::make(1));
            assert_eq!(ts.poll_test(), Some(TestId::make(2)));
            ts.mark_ok(TestId::make(2));
            assert_eq!(ts.poll_test(), Some(TestId::make(3)));
            assert_eq!(ts.poll_test(), None);
        }
    }
}
