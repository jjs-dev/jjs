mod group;

use crate::cfg::Config;
use group::Group;
use invoker_api::{
    valuer_proto::{
        JudgeLog, JudgeLogKind, ProblemInfo, SubtaskVisibleComponents, TestVisibleComponents,
    },
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

#[derive(Copy, Clone, Eq, PartialEq)]
enum GroupVisPreset {
    Full,
    Brief,
    Hidden,
}

impl GroupVisPreset {
    fn subtask_flags_for(self, k: JudgeLogKind) -> SubtaskVisibleComponents {
        let mut out = SubtaskVisibleComponents::empty();
        if self == GroupVisPreset::Full || k == JudgeLogKind::Full {
            out |= SubtaskVisibleComponents::all();
        }
        if self == GroupVisPreset::Brief || k == JudgeLogKind::Full {
            out |= SubtaskVisibleComponents::SCORE;
        }
        out
    }

    fn test_flags_for(self, k: JudgeLogKind) -> TestVisibleComponents {
        let mut out = TestVisibleComponents::empty();
        if self == GroupVisPreset::Full || k == JudgeLogKind::Full {
            out |= TestVisibleComponents::all();
        }
        if self == GroupVisPreset::Brief {
            out |= TestVisibleComponents::STATUS | TestVisibleComponents::RESOURCE_USAGE;
        }
        out
    }

    fn is_visible_for(self, k: JudgeLogKind) -> bool {
        match self {
            GroupVisPreset::Brief | GroupVisPreset::Full => true,
            GroupVisPreset::Hidden => k == JudgeLogKind::Full,
        }
    }
}

impl Fiber {
    pub(crate) fn new(cfg: &Config, problem_info: &ProblemInfo, kind: JudgeLogKind) -> Fiber {
        let mut groups = Vec::new();
        let mut visible_tests = HashSet::new();
        let mut skipped_groups = HashSet::new();
        for (i, group_cfg) in cfg.groups.iter().enumerate() {
            let vis_preset = match group_cfg.feedback {
                crate::cfg::FeedbackKind::Brief => GroupVisPreset::Brief,
                crate::cfg::FeedbackKind::Full => GroupVisPreset::Full,
                crate::cfg::FeedbackKind::Hidden => GroupVisPreset::Hidden,
            };
            if !vis_preset.is_visible_for(kind) {
                skipped_groups.insert(i);
                continue;
            }
            let mut grp = Group::new();
            grp.set_id(NonZeroU32::new((i + 1) as u32).unwrap());
            let mut tests = Vec::new();
            for (i, test_tag) in problem_info.tests.iter().enumerate() {
                if test_tag == group_cfg.tests_tag() {
                    tests.push((i + 1) as u32);
                }
            }
            visible_tests.extend(tests.iter().map(|test_id| pom::TestId::make(*test_id)));
            grp.add_tests(tests);

            grp.set_tests_vis(vis_preset.test_flags_for(kind))
                .set_group_vis(vis_preset.subtask_flags_for(kind));
            grp.set_score(group_cfg.score);
            for dep in &group_cfg.deps {
                let group_id = cfg.get_group(dep).expect("invalid config");
                if skipped_groups.contains(&group_id) {
                    continue;
                }
                grp.add_dep(group_id as u32);
            }
            if !group_cfg.run_to_first_failure {
                grp.set_run_all_tests();
            }

            grp.freeze();

            groups.push(grp);
        }
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
        for &i in &self.active_groups {
            let g = &mut self.groups[i];
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
        debug!("live score: {}", cur_live_score);
        if cur_live_score != self.last_live_score {
            info!(
                "live score updated: old={}, cur={}",
                self.last_live_score, cur_live_score
            );
            self.last_live_score = cur_live_score;
            return FiberReply::LiveScore {
                score: cur_live_score,
            };
        }
        let mut new_active_groups = Vec::new();
        for &i in &self.active_groups {
            let g = &self.groups[i];
            let is_passed = g.is_passed();
            let is_failed = g.is_failed();
            if is_passed || is_failed {
                info!("group {} is finished", i);
            } else {
                new_active_groups.push(i);
            }
            assert!(!(is_passed && is_failed));
            if g.is_passed() {
                debug!("group {} is passed", i);
                for group in &mut self.groups {
                    group.on_group_pass(i as u32);
                }
            } else if g.is_failed() {
                let mut queue = vec![i as u32];
                while let Some(k) = queue.pop() {
                    debug!("group {} is failed", k);
                    for (j, group) in self.groups.iter_mut().enumerate() {
                        if !group.is_waiting() {
                            continue;
                        }
                        group.on_group_fail(k as u32);
                        if group.is_skipped() {
                            queue.push(j as u32);
                        }
                    }
                }
            }
        }
        self.active_groups = new_active_groups;
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

    fn make_fiber(cfg: &str, problem_info: &[&str], kind: JudgeLogKind) -> Fiber {
        Fiber::new(
            &serde_yaml::from_str(cfg).unwrap(),
            &ProblemInfo {
                tests: problem_info.iter().map(ToString::to_string).collect(),
            },
            kind,
        )
    }
    #[test]
    fn simple() {
        simple_logger::SimpleLogger::new().init().ok();
        let mut f = make_fiber(
            "
groups:
  - name: samples
    feedback: full
    score: 0
  - name: online
    feedback: brief
    score: 60
    deps: 
      - samples
  - name: offline
    feedback: hidden
    score: 40
    deps: 
      - online        
        ",
            &["samples", "online", "offline"],
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
                    components: TestVisibleComponents::all()
                },
                JudgeLogTestRow {
                    test_id: TestId::make(2),
                    status: crate::status_util::make_ok_status(),
                    components: TestVisibleComponents::all()
                },
                JudgeLogTestRow {
                    test_id: TestId::make(3),
                    status: crate::status_util::make_err_status(),
                    components: TestVisibleComponents::all()
                },
            ],
        );
        assert_eq!(
            judge_log.subtasks.split_off(0),
            vec![
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(1),
                    score: 0,
                    components: SubtaskVisibleComponents::all()
                },
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(2),
                    score: 60,
                    components: SubtaskVisibleComponents::all()
                },
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(3),
                    score: 0,
                    components: SubtaskVisibleComponents::all()
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
