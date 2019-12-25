use super::*;
use util::{make_err_status, make_ok_status};

#[derive(Debug)]
enum MockItem {
    Resp(ValuerResponse),
    Notify(Option<TestDoneNotification>),
}
struct MockDriver {
    items: VecDeque<MockItem>,
    problem_info: ProblemInfo,
}
impl MockDriver {
    fn new(problem_info: ProblemInfo) -> Self {
        Self {
            items: VecDeque::new(),
            problem_info,
        }
    }

    fn add_response(&mut self, resp: ValuerResponse) -> &mut Self {
        self.items.push_back(MockItem::Resp(resp));
        self
    }

    fn add_notify(&mut self, notify: TestDoneNotification) -> &mut Self {
        self.items.push_back(MockItem::Notify(Some(notify)));
        self
    }

    fn add_none_notify(&mut self) -> &mut Self {
        self.items.push_back(MockItem::Notify(None));
        self
    }

    fn next_event(&mut self, what: impl std::fmt::Debug) -> MockItem {
        self.items
            .pop_front()
            .unwrap_or_else(|| panic!("on {:?}: event deque drained", what))
    }

    fn exec(&mut self) {
        let val = SimpleValuer::new(self, &crate::cfg::Config { open_test_count: 1 }).unwrap();
        val.exec().unwrap();
    }
}

impl ValuerDriver for MockDriver {
    fn problem_info(&mut self) -> Result<ProblemInfo> {
        Ok(self.problem_info.clone())
    }

    fn send_command(&mut self, cmd: &ValuerResponse) -> Result<()> {
        match self.next_event(cmd) {
            MockItem::Resp(expected) => assert_eq!(&expected, cmd),
            ev => panic!("send_command({:?}): expected {:?} instead", cmd, ev),
        }
        Ok(())
    }

    fn poll_notification(&mut self) -> Result<Option<TestDoneNotification>> {
        match self.next_event("poll_notification") {
            MockItem::Notify(notify) => Ok(notify),
            ev => panic!("poll_notification: queue contains {:?} instead", ev),
        }
    }
}

// there two tests check low level interaction details
mod low_level {
    use super::*;
    #[test]
    fn simple_ok() {
        MockDriver::new(ProblemInfo { test_count: 2 })
            .add_none_notify()
            .add_response(ValuerResponse::Test {
                test_id: TestId::make(1),
                live: true,
            })
            .add_notify(TestDoneNotification {
                test_id: TestId::make(1),
                test_status: make_ok_status(),
            })
            .add_none_notify()
            .add_response(ValuerResponse::Test {
                test_id: TestId::make(2),
                live: false,
            })
            .add_notify(TestDoneNotification {
                test_id: TestId::make(2),
                test_status: make_ok_status(),
            })
            .add_response(ValuerResponse::Finish {
                score: 2,
                treat_as_full: true,
                judge_log: invoker_api::valuer_proto::JudgeLog {
                    name: "todo".to_string(),
                    tests: vec![],
                    subtasks: vec![],
                },
            })
            .exec();
    }

    #[test]
    fn status_err() {
        MockDriver::new(ProblemInfo { test_count: 1 })
            .add_none_notify()
            .add_response(ValuerResponse::Test {
                test_id: TestId::make(1),
                live: true,
            })
            .add_notify(TestDoneNotification {
                test_id: TestId::make(1),
                test_status: make_err_status(),
            })
            .add_response(ValuerResponse::Finish {
                score: 0,
                treat_as_full: false,
                judge_log: invoker_api::valuer_proto::JudgeLog {
                    name: "todo".to_string(),
                    tests: vec![],
                    subtasks: vec![],
                },
            })
            .exec();
    }
}
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
