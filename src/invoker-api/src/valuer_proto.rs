//! Defines types used to interact between invoker and valuer
use crate::Status;
use bitflags::bitflags;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgeLogTestRow {
    pub test_id: pom::TestId,
    pub status: Status,
    pub components: TestVisibleComponents,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct SubtaskId(std::num::NonZeroU32);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgeLogSubtaskRow {
    pub subtask_id: SubtaskId,
    pub score: u32,
    pub components: SubtaskVisibleComponents,
}

/// Judge log from valuer POV
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgeLog {
    pub name: String,
    pub tests: Vec<JudgeLogTestRow>,
    pub subtasks: Vec<JudgeLogSubtaskRow>,
    pub compile_stdout: String,
    pub compile_stderr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestDoneNotification {
    pub test_id: u32,
    pub test_status: Status,
}

#[derive(Serialize, Deserialize)]
pub enum ValuerResponse {
    Test {
        test_id: u32,
        live: bool,
    },
    Finish {
        score: u32,
        treat_as_full: bool,
        judge_log: JudgeLog,
    },
    LiveScore {
        score: u32,
    },
}
