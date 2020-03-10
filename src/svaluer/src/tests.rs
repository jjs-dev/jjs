use super::*;
use invoker_api::{
    valuer_proto::{
        JudgeLog, JudgeLogSubtaskRow, JudgeLogTestRow, SubtaskId, SubtaskVisibleComponents,
        TestVisibleComponents,
    },
    Status,
};
use status_util::{make_err_status, make_ok_status};
use std::collections::VecDeque;

#[derive(Debug)]
struct TestMock {
    test_id: TestId,
    live: bool,
    status: Status,
}

#[derive(Debug)]
struct MockDriver {
    tests: VecDeque<TestMock>,
    pending_notifications: VecDeque<TestDoneNotification>,
    live_scores: VecDeque<u32>,
    problem_info: ProblemInfo,
    judge_logs: Vec<JudgeLog>,
}
impl MockDriver {
    fn new(problem_info: ProblemInfo) -> Self {
        Self {
            tests: VecDeque::new(),
            problem_info,
            live_scores: VecDeque::new(),
            pending_notifications: VecDeque::new(),
            judge_logs: Vec::new(),
        }
    }

    fn add_test(&mut self, test_id: u32, live: bool, ok: bool) -> &mut Self {
        let mock = TestMock {
            test_id: TestId::make(test_id),
            live,
            status: if ok {
                make_ok_status()
            } else {
                make_err_status()
            },
        };
        self.tests.push_back(mock);
        self
    }

    fn add_judge_log(&mut self, judge_log: JudgeLog) -> &mut Self {
        self.judge_logs.push(judge_log);
        self
    }

    fn add_live_score(&mut self, score: u32) -> &mut Self {
        self.live_scores.push_back(score);
        self
    }

    fn check_finish(&mut self) {
        if !self.pending_notifications.is_empty() {
            panic!("not all notifications are delivered");
        }
        if !self.live_scores.is_empty() {
            panic!("not all live scores were emitted");
        }
        if !self.tests.is_empty() {
            panic!("not all tests were executed");
        }
        if let Some(judge_log) = self.judge_logs.first() {
            panic!("judge log {:?} was not emitted", judge_log.kind);
        }
    }

    fn check_live_score(&mut self, score: u32) {
        match self.live_scores.pop_front() {
            Some(expected) => {
                if expected != score {
                    panic!(
                        "expected live score {}, but valuer gave {}",
                        expected, score
                    );
                }
            }
            None => panic!("no more live scores expected, but got {}", score),
        }
    }

    fn check_test(&mut self, test_id: TestId, live: bool) {
        match self.tests.pop_front() {
            Some(mock) => {
                if mock.test_id != test_id {
                    panic!(
                        "expected {} to be next test, but got {} instead",
                        mock.test_id.get(),
                        test_id.get()
                    );
                }
                if mock.live != live {
                    panic!("live flag differs: expected {}, actual {}", mock.live, live);
                }
                self.pending_notifications.push_back(TestDoneNotification {
                    test_id: mock.test_id,
                    test_status: mock.status,
                })
            }
            None => panic!(
                "No more test runs expected, but got request for {}",
                test_id.get()
            ),
        }
    }

    fn check_judge_log(&mut self, judge_log: &JudgeLog) {
        let mut iter = self
            .judge_logs
            .drain_filter(|log| log.kind == judge_log.kind);
        let expected = match iter.next() {
            Some(e) => e,
            None => panic!("Judge log of kind {:?} is not expected", judge_log.kind),
        };
        if let Some(dupe) = iter.next() {
            panic!("Invalid test data: duplicated judge log: {:?}", dupe.kind);
        }
        assert_eq!(expected.tests, judge_log.tests);
        assert_eq!(expected.subtasks, judge_log.subtasks);
        assert_eq!(expected.score, judge_log.score);
        assert_eq!(expected.kind, judge_log.kind);
        assert_eq!(expected.is_full, judge_log.is_full);
        // In case new field is added, of course an assert should be added.
        // But as additional check, compare full logs.
        assert_eq!(&expected, judge_log);
    }

    fn exec(&mut self, cfg: impl AsRef<str>) {
        simple_logger::init().ok();
        let cfg = cfg.as_ref();
        let cfg = serde_yaml::from_str(cfg).expect("failed to parse config");
        let valuer = SimpleValuer::new(self, &cfg).unwrap();
        valuer.exec().unwrap();
    }
}

impl ValuerDriver for MockDriver {
    fn problem_info(&mut self) -> Result<ProblemInfo> {
        Ok(self.problem_info.clone())
    }

    fn send_command(&mut self, cmd: &ValuerResponse) -> Result<()> {
        match cmd {
            ValuerResponse::Finish => self.check_finish(),
            ValuerResponse::JudgeLog(judge_log) => self.check_judge_log(judge_log),
            ValuerResponse::LiveScore { score } => self.check_live_score(*score),
            ValuerResponse::Test { test_id, live } => self.check_test(*test_id, *live),
        }
        Ok(())
    }

    fn poll_notification(&mut self) -> Result<Option<TestDoneNotification>> {
        Ok(self.pending_notifications.pop_front())
    }
}

mod simple {
    use super::*;
    #[test]
    fn simple_ok() {
        let full_log = JudgeLog {
            is_full: true,
            kind: JudgeLogKind::Full,
            tests: vec![
                JudgeLogTestRow {
                    test_id: TestId::make(1),
                    status: make_ok_status(),
                    components: TestVisibleComponents::all(),
                },
                JudgeLogTestRow {
                    test_id: TestId::make(2),
                    status: make_ok_status(),
                    components: TestVisibleComponents::all(),
                },
            ],
            subtasks: vec![
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(1),
                    score: 64,
                    components: SubtaskVisibleComponents::SCORE,
                },
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(2),
                    score: 36,
                    components: SubtaskVisibleComponents::SCORE,
                },
            ],
            score: 100,
        };
        let mut contestant_log = full_log.clone();
        contestant_log.kind = JudgeLogKind::Contestant;
        contestant_log.subtasks.pop();
        contestant_log.tests.pop();
        contestant_log.tests[0].components = TestVisibleComponents::STATUS;
        contestant_log.score = 64;
        MockDriver::new(ProblemInfo {
            tests: vec!["online".to_string(), "offline".to_string()],
        })
        .add_test(1, true, true)
        .add_test(2, false, true)
        .add_judge_log(full_log)
        .add_judge_log(contestant_log)
        .add_live_score(64)
        .exec(
            "
groups:
  - name: online
    feedback: brief
    score: 64
  - name: offline
    feedback: hidden
    score: 36
    deps:
      - online
            ",
        );
    }

    #[test]
    fn status_err() {
        let full_log = JudgeLog {
            is_full: false,
            kind: JudgeLogKind::Full,
            tests: vec![JudgeLogTestRow {
                test_id: TestId::make(1),
                status: make_err_status(),
                components: TestVisibleComponents::all(),
            }],
            subtasks: vec![
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(1),
                    score: 0,
                    components: SubtaskVisibleComponents::all(),
                },
                JudgeLogSubtaskRow {
                    subtask_id: SubtaskId::make(2),
                    score: 0,
                    components: SubtaskVisibleComponents::all(),
                },
            ],
            score: 0,
        };
        let mut contestant_log = full_log.clone();
        contestant_log.kind = JudgeLogKind::Contestant;
        MockDriver::new(ProblemInfo {
            tests: vec!["samples".to_string(), "online".to_string()],
        })
        .add_test(1, true, false)
        .add_judge_log(full_log)
        .add_judge_log(contestant_log)
        .exec(
            "
groups:
  - name: samples
    score: 0
    feedback: full
  - name: online
    score: 100
    feedback: brief
    deps:
      - samples    
                ",
        );
    }
}
