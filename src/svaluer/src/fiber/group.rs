use either::{Left, Right};
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
struct RunningState {
    queued_tests: BTreeSet<TestId>,
    succeeded_tests: BTreeSet<(TestId, Status)>,
    failed_tests: BTreeSet<(TestId, Status)>,
    running_tests: BTreeSet<TestId>,
}

#[derive(Debug)]
struct WaitingState {
    deps: BTreeSet<u32>,
}

#[derive(Debug)]
struct SkippedState {
    failed_dep: u32,
}

#[derive(Debug)]
struct FinishedState {
    score: u32,
    success: bool,
    tests: Vec<(TestId, Status)>,
}

#[derive(Debug)]
enum State {
    Building,
    Running(RunningState),
    Waiting(WaitingState),
    Skipped(SkippedState),
    Finished(FinishedState),
}

#[derive(Debug)]
pub(crate) struct Group {
    id: SubtaskId,
    dep_groups: Vec<u32>,
    test_vis_flags: TestVisibleComponents,
    subtask_vis_flags: SubtaskVisibleComponents,
    run_all_tests: bool,
    state: State,
    tests: Vec<TestId>,
    score: u32,
}

impl Group {
    pub(crate) fn new() -> Group {
        Group {
            id: SubtaskId(std::num::NonZeroU32::new(u32::max_value()).unwrap()),
            dep_groups: Vec::new(),
            test_vis_flags: TestVisibleComponents::empty(),
            subtask_vis_flags: SubtaskVisibleComponents::empty(),
            run_all_tests: false,
            state: State::Building,
            tests: Vec::new(),
            score: 0,
        }
    }

    fn check_mutable(&self) {
        assert!(matches!(self.state, State::Building))
    }

    pub(crate) fn set_id(&mut self, id: std::num::NonZeroU32) -> &mut Self {
        self.check_mutable();
        self.id = SubtaskId(id);
        self
    }

    pub(crate) fn add_tests(&mut self, range: impl IntoIterator<Item = u32>) -> &mut Self {
        self.check_mutable();
        self.tests.extend(range.into_iter().map(TestId::make)); // TODO: do not panic
        self
    }

    pub(crate) fn add_dep(&mut self, dep_id: u32) -> &mut Self {
        self.check_mutable();
        self.dep_groups.push(dep_id);
        self
    }

    pub(crate) fn set_score(&mut self, score: u32) -> &mut Self {
        self.check_mutable();
        self.score = score;
        self
    }

    pub(crate) fn set_tests_vis(
        &mut self,
        vis: invoker_api::valuer_proto::TestVisibleComponents,
    ) -> &mut Self {
        self.check_mutable();
        self.test_vis_flags = vis;
        self
    }

    pub(crate) fn set_group_vis(
        &mut self,
        vis: invoker_api::valuer_proto::SubtaskVisibleComponents,
    ) -> &mut Self {
        self.check_mutable();
        self.subtask_vis_flags = vis;
        self
    }

    pub(crate) fn set_run_all_tests(&mut self) -> &mut Self {
        self.check_mutable();
        self.run_all_tests = true;
        self
    }

    pub(crate) fn freeze(&mut self) {
        self.state = State::Waiting(WaitingState {
            deps: self.dep_groups.iter().copied().collect(),
        });
        self.maybe_stop_waiting();
    }
}

impl Group {
    fn finished(&self) -> Option<bool> {
        match &self.state {
            State::Finished(state) => Some(state.success),
            _ => None,
        }
    }

    pub(crate) fn is_passed(&self) -> bool {
        self.finished() == Some(true)
    }

    pub(crate) fn is_failed(&self) -> bool {
        self.finished() == Some(false)
    }

    pub(crate) fn running_tests(&self) -> u32 {
        match &self.state {
            State::Running(state) => state.running_tests.len() as u32,
            _ => 0,
        }
    }

    pub(crate) fn list_running_tests(&self) -> impl Iterator<Item = TestId> + '_ {
        match &self.state {
            State::Running(state) => Left(state.running_tests.iter().copied()),
            _ => Right(std::iter::empty()),
        }
    }

    fn maybe_stop_waiting(&mut self) {
        let state = match &mut self.state {
            State::Waiting(state) => state,
            _ => unreachable!(),
        };
        if state.deps.is_empty() {
            self.state = State::Running(RunningState {
                queued_tests: self.tests.iter().copied().collect(),
                failed_tests: BTreeSet::new(),
                succeeded_tests: BTreeSet::new(),
                running_tests: BTreeSet::new(),
            });
        }
    }

    pub(crate) fn on_group_pass(&mut self, other_group_id: u32) {
        let state = match &mut self.state {
            State::Waiting(state) => state,
            _ => return,
        };
        if state.deps.remove(&other_group_id) {
            debug!("group {:?}: dep {} passed", self.id, other_group_id);
            self.maybe_stop_waiting();
        }
    }

    pub(crate) fn on_group_fail(&mut self, other_group_id: u32) {
        let state = match &mut self.state {
            State::Waiting(state) => state,
            _ => return,
        };
        if !state.deps.contains(&other_group_id) {
            // that group was not required, so we ignore this failure
            return;
        }
        debug!("group {:?}: dep {} failed", self.id, other_group_id);
        self.state = State::Skipped(SkippedState {
            failed_dep: other_group_id,
        });
    }

    /// Returns next test from this group that can be executed
    pub(crate) fn pop_test(&mut self) -> Option<TestId> {
        debug!("Group {:?}: searching for test", self.id);
        // we can't run if some deps are running or failed
        let state = match &mut self.state {
            State::Building => unreachable!(),
            State::Finished(_) | State::Skipped(_) => {
                debug!("Returning None: group has been done");
                return None;
            }
            State::Waiting(_) => {
                debug!("Returning None: some deps still not finished");
                return None;
            }
            State::Running(state) => state,
        };
        if !state.running_tests.is_empty() && !self.run_all_tests {
            debug!("Returning None: run_all_tests=false, and a test is already running");
            return None;
        }
        if let Some(t) = state.queued_tests.iter().next().copied() {
            debug!("found test: {}", t);
            state.queued_tests.remove(&t);
            state.running_tests.insert(t);
            Some(t)
        } else {
            debug!("Queue is empty");
            None
        }
    }

    fn running_state(&mut self) -> &mut RunningState {
        match &mut self.state {
            State::Running(state) => state,
            other => panic!("expected RunningState, got {:?}", other),
        }
    }

    fn mark_test_fail(&mut self, test_id: TestId, status: Status) {
        let id = self.id;
        let must_run_all_tests = self.run_all_tests;
        let state = self.running_state();
        if state.failed_tests.is_empty() {
            debug!("group {:?} is now failed", id);
        }
        state.failed_tests.insert((test_id, status));
        if !must_run_all_tests {
            // no other tests should be run
            state.queued_tests.clear();
        }
    }

    fn mark_test_ok(&mut self, test_id: TestId, status: Status) {
        self.running_state()
            .succeeded_tests
            .insert((test_id, status));
    }

    fn maybe_finish(&mut self) {
        let state = self.running_state();
        if state.queued_tests.is_empty() && state.running_tests.is_empty() {
            let success = state.failed_tests.is_empty();
            let failed_tests = std::mem::take(&mut state.failed_tests);
            let succeeded_tests = std::mem::take(&mut state.succeeded_tests);
            let score = if success { self.score } else { 0 };
            self.state = State::Finished(FinishedState {
                score,
                success,
                tests: failed_tests.into_iter().chain(succeeded_tests).collect(),
            })
        }
    }

    pub(crate) fn on_test_done(&mut self, test_id: TestId, status: Status) {
        let state = match &mut self.state {
            State::Running(state) => state,
            _ => return,
        };
        if !state.running_tests.remove(&test_id) {
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
        self.maybe_finish();
    }

    pub(crate) fn update_judge_log(&self, log: &mut JudgeLog) {
        let state = match &self.state {
            State::Finished(state) => state,
            State::Skipped(_) => {
                log.subtasks.push(JudgeLogSubtaskRow {
                    components: self.subtask_vis_flags,
                    score: 0,
                    subtask_id: self.id,
                });
                return;
            }
            other => panic!("update_judge_log: unexpected state {:?}", other),
        };
        let self_score = self.score();
        log.score += self_score;
        let subtask_entry = JudgeLogSubtaskRow {
            components: self.subtask_vis_flags,
            score: self_score,
            subtask_id: self.id,
        };
        log.subtasks.push(subtask_entry);
        for (test, status) in &state.tests {
            let row = JudgeLogTestRow {
                components: self.test_vis_flags,
                test_id: *test,
                status: status.clone(),
            };
            log.tests.push(row);
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
        g.freeze();

        assert_eq!(g.pop_test(), Some(TestId::make(1)));
        g.on_test_done(TestId::make(1), st());
        assert_eq!(g.pop_test(), Some(TestId::make(2)));
        g.on_test_done(TestId::make(2), st());
        assert_eq!(g.pop_test(), Some(TestId::make(3)));
        assert_eq!(g.pop_test(), None);
    }
}
