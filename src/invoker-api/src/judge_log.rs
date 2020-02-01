//! Judge log stored in FS
use crate::{
    valuer_proto::{JudgeLogKind, SubtaskId},
    Status, StatusKind,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgeLogTestRow {
    pub test_id: pom::TestId,
    pub status: Option<Status>,
    pub test_stdin: Option<String>,
    pub test_stdout: Option<String>,
    pub test_stderr: Option<String>,
    pub test_answer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgeLogSubtaskRow {
    pub subtask_id: SubtaskId,
    pub score: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgeLog {
    pub kind: JudgeLogKind,
    pub tests: Vec<JudgeLogTestRow>,
    pub subtasks: Vec<JudgeLogSubtaskRow>,
    pub compile_stdout: String,
    pub compile_stderr: String,
    pub score: u32,
    pub is_full: bool,
    pub status: Status,
}

#[derive(Debug, Clone, Serialize)]
pub struct JudgeLogs(pub Vec<JudgeLog>);

impl JudgeLogs {
    pub fn full_log(&self) -> Option<&JudgeLog> {
        self.0.iter().find(|log| log.kind == JudgeLogKind::Full)
    }

    pub fn full_status(&self) -> Option<&Status> {
        self.full_log()
            .or_else(|| self.0.get(0))
            .map(|log| &log.status)
    }
}

impl Default for JudgeLog {
    fn default() -> Self {
        Self {
            kind: JudgeLogKind::Contestant,
            tests: vec![],
            subtasks: vec![],
            compile_stdout: String::new(),
            compile_stderr: String::new(),
            score: 0,
            is_full: false,
            status: Status {
                code: "".to_string(),
                kind: StatusKind::NotSet,
            },
        }
    }
}
