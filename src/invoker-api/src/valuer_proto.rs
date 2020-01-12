//! Defines types used to interact between invoker and valuer
use crate::Status;
use bitflags::bitflags;
use pom::TestId;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct TestVisibleComponents: u32 {
        /// Test input data
        const TEST_DATA = 1;
        /// Solution stdout & stderr
        const OUTPUT = 2;
        /// Test answer
        const ANSWER = 4;
        /// Test status
        const STATUS = 8;
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct SubtaskVisibleComponents: u32 {
        /// Score gained for this subtask
        const SCORE = 1;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct JudgeLogTestRow {
    pub test_id: pom::TestId,
    pub status: Status,
    pub components: TestVisibleComponents,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub struct SubtaskId(pub std::num::NonZeroU32);

impl SubtaskId {
    pub fn make(n: u32) -> SubtaskId {
        SubtaskId(std::num::NonZeroU32::new(n).expect("SubtaskId cannot be maked from 0"))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct JudgeLogSubtaskRow {
    pub subtask_id: SubtaskId,
    pub score: u32,
    pub components: SubtaskVisibleComponents,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
pub enum JudgeLogKind {
    /// Contains all tests.
    /// Test can be omitted, if staring it was speculation.
    Full,
    /// Contains judge log for contestant
    /// Valuer should respect various restrictions specified in config.
    Contestant,
}

/// Judge log from valuer POV
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct JudgeLog {
    pub kind: JudgeLogKind,
    pub tests: Vec<JudgeLogTestRow>,
    pub subtasks: Vec<JudgeLogSubtaskRow>,
    pub score: u32,
    pub is_full: bool,
}

impl Default for JudgeLog {
    fn default() -> JudgeLog {
        JudgeLog {
            kind: JudgeLogKind::Contestant,
            tests: Vec::new(),
            subtasks: Vec::new(),
            score: 0,
            is_full: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemInfo {
    pub test_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TestDoneNotification {
    pub test_id: TestId,
    pub test_status: Status,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum ValuerResponse {
    Test {
        test_id: TestId,
        live: bool,
    },
    /// Sent when judge log ready
    /// Judge log of each kind must be sent at most once
    JudgeLog(JudgeLog),
    Finish,
    LiveScore {
        score: u32,
    },
}
