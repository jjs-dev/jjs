mod group;

use crate::cfg::Config;
use group::Group;
use invoker_api::{
    valuer_proto::{JudgeLog, JudgeLogKind, ProblemInfo},
    Status,
};
use log::{debug, info};
use pom::TestId;
use std::{collections::HashSet, num::NonZeroU32};

/// Creates single JudgeLog
/// SValuer works by aggegating several fibers (one per judgelog kind).
#[derive(Debug)]
pub(crate) struct Fiber {
    kind: JudgeLogKind,
    /// If test is not in this set, it will not be included into judge log.
    visible_tests: HashSet<TestId>,
    // contains indices for `groups`
    active_groups: Vec<usize>,
    groups: Vec<Group>,
    finished: bool,
    last_live_score: u32,
}

// TODO: consider unifying with ValuerResponse
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum FiberReply {
    Test { test_id: TestId },
    Finish(JudgeLog),
    None,
    LiveScore { score: u32 },
}

impl Fiber {
    pub(crate) fn new(cfg: &Config, problem_info: &ProblemInfo, kind: JudgeLogKind) -> Fiber {
        let mut groups = Vec::new();

        let mut samples_group = Group::new();
        samples_group
            .add_tests(1..=cfg.samples_count)
            .set_id(NonZeroU32::new(1).unwrap());
        groups.push(samples_group);

        let mut online_tests_group = Group::new();
        online_tests_group
            .add_dep(0)
            .set_id(NonZeroU32::new(2).unwrap());
        {
            let first = cfg.samples_count + 1;
            let last = match cfg.open_tests_count {
                Some(cnt) => first + cnt - 1,
                None => problem_info.test_count,
            };
            assert!(last <= problem_info.test_count);
            online_tests_group.add_tests(first..=last);
            if cfg.open_tests_count.is_some() {
                online_tests_group.set_score(cfg.open_tests_score.unwrap());
            } else {
                online_tests_group.set_score(100);
            }
        }
        groups.push(online_tests_group);
        if kind == JudgeLogKind::Full {
            if let Some(open_test_count) = cfg.open_tests_count {
                let mut offline_tests_group = Group::new();
                offline_tests_group
                    .add_dep(1)
                    .set_id(NonZeroU32::new(3).unwrap());
                let first = open_test_count + cfg.samples_count + 1;
                let last = problem_info.test_count;
                offline_tests_group.add_tests(first..=last);
                offline_tests_group.set_score(100 - cfg.open_tests_score.unwrap());
                groups.push(offline_tests_group);
            }
        }

        let test_count = match kind {
            JudgeLogKind::Contestant => cfg.open_tests_count.unwrap_or(problem_info.test_count),
            JudgeLogKind::Full => problem_info.test_count,
        };
        let visible_tests = (1..=test_count).map(TestId::make).collect();

        Fiber {
            kind,
            visible_tests,
            active_groups: (0..groups.len()).collect(),
            finished: false,
            groups,
            last_live_score: 0,
        }
    }

    pub(crate) fn add(&mut self, notification: &invoker_api::valuer_proto::TestDoneNotification) {
        if !self.visible_tests.contains(&notification.test_id) {
            return;
        }
        if self.finished {
            panic!("Fiber is finished, but got notification {:?}", notification);
        }
        self.add_test(notification.test_id, &notification.test_status);
    }

    pub(crate) fn kind(&self) -> JudgeLogKind {
        self.kind
    }

    fn emit_judgelog(&mut self) -> FiberReply {
        debug!("Emitting {:?} judge log", self.kind);
        let is_full = self.groups.iter().all(|g| g.is_passed());
        let mut judge_log = JudgeLog {
            kind: self.kind,
            tests: vec![],
            subtasks: vec![],
            is_full,
            score: 0,
        };
        for (i, g) in self.groups.iter().enumerate() {
            debug!("extending judge log with group {}", i);
            g.update_judge_log(&mut judge_log);
        }

        FiberReply::Finish(judge_log)
    }

    fn poll_groups_for_tests(&mut self) -> Option<TestId> {
        for (i, g) in self.groups.iter_mut().enumerate() {
            if let reply @ Some(_) = g.pop_test() {
                debug!("group {} returned {}", i, reply.unwrap());
                return reply;
            }
            debug!("group {} is not ready to run yet", i);
        }
        None
    }

    pub(crate) fn test_is_live(&self, test: TestId) -> bool {
        self.kind == JudgeLogKind::Contestant && self.visible_tests.contains(&test)
    }

    fn current_score(&self) -> u32 {
        self.groups.iter().map(|g| g.score()).sum()
    }

    fn running_tests(&self) -> u32 {
        self.groups.iter().map(|g| g.running_tests()).sum()
    }

    pub(crate) fn poll(&mut self) -> FiberReply {
        debug!("Fiber {:?}: poll iteration", self.kind);
        if self.finished {
            debug!("Returning none: already finished");
            return FiberReply::None;
        }
        let cur_live_score = self.current_score();
        if cur_live_score != self.last_live_score {
            debug!(
                "live score updated: old={}, cur={}",
                self.last_live_score, cur_live_score
            );
            self.last_live_score = cur_live_score;
            return FiberReply::LiveScore {
                score: cur_live_score,
            };
        }
        let mut del_from_active_groups = Vec::new();
        for &i in &self.active_groups {
            let g = &self.groups[i];
            let is_passed = g.is_passed();
            let is_failed = g.is_failed();
            if is_passed || is_failed {
                info!("group {} is finished", i);
                del_from_active_groups.push(i);
            }
            assert!(!(is_passed && is_failed));
            if g.is_passed() {
                debug!("group {} is passed", i);
                for group in &mut self.groups {
                    group.on_group_pass(i as u32);
                }
            } else if g.is_failed() {
                debug!("group {} is failed", i);
                for group in &mut self.groups {
                    group.on_group_fail(i as u32);
                }
            }
        }
        if let Some(test_id) = self.poll_groups_for_tests() {
            debug!("got test from groups: {}", test_id);
            FiberReply::Test { test_id }
        } else if self.running_tests() == 0 {
            debug!(
                "this fiber is finished, emitting judge log of kind {:?}",
                self.kind
            );
            self.finished = true;
            self.emit_judgelog()
        } else {
            let running_test = self
                .groups
                .iter()
                .map(|g| g.list_running_tests())
                .flatten()
                .next()
                .expect("unreachable: running_tests() > 0, but no test found");
            debug!(
                "no updates yet, waiting for running tests, incl. {:?}",
                running_test
            );

            // let's wait
            FiberReply::None
        }
    }

    fn add_test(&mut self, test: TestId, status: &Status) {
        debug!("processing status {:?} for test {}", status, test);
        if !self.visible_tests.contains(&test) {
            debug!("skipping: test is not visible");
            return;
        }
        for g in &mut self.groups {
            g.on_test_done(test, status.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use invoker_api::valuer_proto::{
        JudgeLogSubtaskRow, JudgeLogTestRow, SubtaskId, SubtaskVisibleComponents,
        TestVisibleComponents,
    };
    #[test]
    fn simple() {
        let mut f = Fiber::new(
            &Config {
                open_tests_count: Some(1),
                open_tests_score: Some(60),
                samples_count: 1,
            },
            &ProblemInfo { test_count: 3 },
            JudgeLogKind::Full,
        );
        assert_eq!(
            f.poll(),
            FiberReply::Test {
                test_id: TestId::make(1)
            }
        );
        assert_eq!(f.poll(), FiberReply::None);
        f.add_test(TestId::make(1), &crate::status_util::make_ok_status());
        assert_eq!(
            f.poll(),
            FiberReply::Test {
                test_id: TestId::make(2)
            }
        );
        assert_eq!(f.poll(), FiberReply::None);
        f.add_test(TestId::make(2), &crate::status_util::make_ok_status());
        assert_eq!(f.poll(), FiberReply::LiveScore { score: 60 });
        assert_eq!(
            f.poll(),
            FiberReply::Test {
                test_id: TestId::make(3)
            }
        );
        assert_eq!(f.poll(), FiberReply::None);
        f.add_test(TestId::make(3), &crate::status_util::make_err_status());
        let mut judge_log = match f.poll() {
            FiberReply::Finish(log) => log,
            oth => panic!("{:?}", oth),
        };
        assert_eq!(
            judge_log.tests.split_off(0),
            vec![
                JudgeLogTestRow {
                    test_id: TestId::make(1),
                    status: crate::status_util::make_ok_status(),
                    components: TestVisibleComponents::empty()
                },
                JudgeLogTestRow {
                    test_id: TestId::make(2),
                    status: crate::status_util::make_ok_status(),
                    components: TestVisibleComponents::empty()
                },
                JudgeLogTestRow {
                    test_id: TestId::make(3),
                    status: crate::status_util::make_err_status(),
                    components: TestVisibleComponents::empty()
                },
            ],
        );
        assert_eq!(
            judge_log.subtasks.split_off(0),
            vec![
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(1),
                    score: 0,
                    components: SubtaskVisibleComponents::empty()
                },
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(2),
                    score: 60,
                    components: SubtaskVisibleComponents::empty()
                },
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(3),
                    score: 0,
                    components: SubtaskVisibleComponents::empty()
                }
            ]
        );
        assert_eq!(
            judge_log,
            JudgeLog {
                is_full: false,
                kind: JudgeLogKind::Full,
                score: 60,
                tests: vec![],
                subtasks: vec![]
            }
        );
    }
}
