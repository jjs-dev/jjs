use invoker_api::{
    valuer_proto::{
        JudgeLog, JudgeLogSubtaskRow, JudgeLogTestRow, SubtaskId, SubtaskVisibleComponents,
        TestVisibleComponents,
    },
    Status,
};
use log::debug;
use pom::TestId;
use std::collections::BTreeSet;

#[derive(Debug)]
pub(crate) struct Group {
    id: SubtaskId,
    deps: BTreeSet<u32>,
    score: u32,
    test_vis_flags: TestVisibleComponents,
    subtask_vis_flags: SubtaskVisibleComponents,
    pending_tests: BTreeSet<TestId>,
    succeeded_tests: BTreeSet<(TestId, Status)>,
    failed_tests: BTreeSet<(TestId, Status)>,
    has_failed_deps: bool,
    running_tests: BTreeSet<TestId>,
    run_all_tests: bool,
}

impl Group {
    pub(crate) fn new() -> Group {
        Group {
            id: SubtaskId(std::num::NonZeroU32::new(u32::max_value()).unwrap()),
            deps: BTreeSet::new(),
            score: 0,
            test_vis_flags: TestVisibleComponents::empty(),
            subtask_vis_flags: SubtaskVisibleComponents::empty(),
            pending_tests: BTreeSet::new(),
            succeeded_tests: BTreeSet::new(),
            failed_tests: BTreeSet::new(),
            has_failed_deps: false,
            running_tests: BTreeSet::new(),
            run_all_tests: false,
        }
    }

    pub(crate) fn set_id(&mut self, id: std::num::NonZeroU32) -> &mut Self {
        self.id = SubtaskId(id);
        self
    }

    pub(crate) fn add_tests(&mut self, range: impl IntoIterator<Item = u32>) -> &mut Self {
        self.pending_tests
            .extend(range.into_iter().map(TestId::make)); // TODO: do not panic
        self
    }

    pub(crate) fn add_dep(&mut self, dep_id: u32) -> &mut Self {
        self.deps.insert(dep_id);
        self
    }

    pub(crate) fn set_score(&mut self, score: u32) -> &mut Self {
        self.score = score;
        self
    }

    pub(crate) fn set_tests_vis(
        &mut self,
        vis: invoker_api::valuer_proto::TestVisibleComponents,
    ) -> &mut Self {
        self.test_vis_flags = vis;
        self
    }

    pub(crate) fn set_group_vis(
        &mut self,
        vis: invoker_api::valuer_proto::SubtaskVisibleComponents,
    ) -> &mut Self {
        self.subtask_vis_flags = vis;
        self
    }

    pub(crate) fn set_run_all_tests(&mut self) -> &mut Self {
        self.run_all_tests = true;
        self
    }
}

impl Group {
    fn is_done(&self) -> bool {
        self.pending_tests.is_empty() && self.running_tests.is_empty()
    }

    pub(crate) fn is_passed(&self) -> bool {
        self.failed_tests.is_empty() && !self.has_failed_deps && self.is_done()
    }

    pub(crate) fn is_failed(&self) -> bool {
        !self.failed_tests.is_empty() || self.has_failed_deps
    }

    pub(crate) fn running_tests(&self) -> u32 {
        self.running_tests.len() as u32
    }

    pub(crate) fn list_running_tests(&self) -> impl Iterator<Item = TestId> + '_ {
        self.running_tests.iter().copied()
    }

    pub(crate) fn on_group_pass(&mut self, other_group_id: u32) {
        if self.deps.remove(&other_group_id) {
            debug!("group {:?}: dep {} passed", self.id, other_group_id);
        }
    }

    pub(crate) fn on_group_fail(&mut self, other_group_id: u32) {
        if !self.deps.contains(&other_group_id) {
            // that group was not required, so we ignore this failure
            return;
        }
        debug!("group {:?}: dep {} failed", self.id, other_group_id);
        self.has_failed_deps = true;
        self.pending_tests.clear();
    }

    /// Returns next test from this group that can be executed
    pub(crate) fn pop_test(&mut self) -> Option<TestId> {
        debug!("Group {:?}: searching for test", self.id);
        // we can't run if some deps are running or failed
        if self.has_failed_deps || !self.deps.is_empty() {
            debug!("Returning None: group has failed or not all deps succeeded");
            return None;
        }
        if !self.running_tests.is_empty() && !self.run_all_tests {
            debug!("Returning None: run_all_tests=false, and a test is already running");
            return None;
        }
        if let Some(t) = self.pending_tests.iter().next().copied() {
            debug!("found test: {}", t);
            self.pending_tests.remove(&t);
            self.running_tests.insert(t);
            Some(t)
        } else {
            debug!("Queue is empty");
            None
        }
    }

    fn mark_test_fail(&mut self, test_id: TestId, status: Status) {
        if self.failed_tests.is_empty() {
            debug!("group {:?} is now failed", self.id);
        }
        self.failed_tests.insert((test_id, status));
        // no other tests should be run
        self.pending_tests.clear();
    }

    fn mark_test_ok(&mut self, test_id: TestId, status: Status) {
        self.succeeded_tests.insert((test_id, status));
    }

    pub(crate) fn on_test_done(&mut self, test_id: TestId, status: Status) {
        debug!("got notificaton for test {}", test_id.get());
        if !self.running_tests.remove(&test_id) {
            return;
        }
        debug!(
            "got test result: test={}, status={:?}",
            test_id.get(),
            status
        );
        if status.kind.is_success() {
            self.mark_test_ok(test_id, status);
        } else {
            self.mark_test_fail(test_id, status);
        }
    }

    pub(crate) fn update_judge_log(&self, log: &mut JudgeLog) {
        let self_score = self.score();
        log.score += self_score;
        let subtask_entry = JudgeLogSubtaskRow {
            components: self.subtask_vis_flags,
            score: self_score,
            subtask_id: self.id,
        };
        log.subtasks.push(subtask_entry);
        assert!(self.pending_tests.is_empty());
        assert!(self.running_tests.is_empty());
        for (test, status) in &self.succeeded_tests {
            let row = JudgeLogTestRow {
                components: self.test_vis_flags,
                test_id: *test,
                status: status.clone(),
            };
            log.tests.push(row);
        }
        for (test, status) in &self.failed_tests {
            log.tests.push(JudgeLogTestRow {
                components: self.test_vis_flags,
                test_id: *test,
                status: status.clone(),
            })
        }
    }

    pub(crate) fn score(&self) -> u32 {
        if self.is_passed() { self.score } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simple() {
        simple_logger::init().ok();
        let st = || Status {
            kind: invoker_api::StatusKind::Accepted,
            code: "MOCK_OK".to_string(),
        };
        let mut g = Group::new();
        g.add_tests(1..=3);
        assert_eq!(g.pop_test(), Some(TestId::make(1)));
        g.on_test_done(TestId::make(1), st());
        assert_eq!(g.pop_test(), Some(TestId::make(2)));
        g.on_test_done(TestId::make(2), st());
        assert_eq!(g.pop_test(), Some(TestId::make(3)));
        assert_eq!(g.pop_test(), None);
    }
}
