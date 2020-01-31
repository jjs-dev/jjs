//! Judge log stored in FS
use invoker_api::{
    valuer_proto::{JudgeLogKind, SubtaskId},
    Status,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct JudgeLogTestRow {
    pub(crate) test_id: pom::TestId,
    pub(crate) status: Option<invoker_api::Status>,
    pub(crate) test_stdin: Option<String>,
    pub(crate) test_stdout: Option<String>,
    pub(crate) test_stderr: Option<String>,
    pub(crate) test_answer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct JudgeLogSubtaskRow {
    pub(crate) subtask_id: SubtaskId,
    pub(crate) score: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct JudgeLog {
    pub(crate) kind: JudgeLogKind,
    pub(crate) tests: Vec<JudgeLogTestRow>,
    pub(crate) subtasks: Vec<JudgeLogSubtaskRow>,
    pub(crate) compile_stdout: String,
    pub(crate) compile_stderr: String,
    pub(crate) score: u32,
    pub(crate) is_full: bool,
    pub(crate) status: invoker_api::Status,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct JudgeLogs(pub(crate) Vec<JudgeLog>);

impl JudgeLogs {
    pub(crate) fn full_log(&self) -> Option<&JudgeLog> {
        self.0.iter().find(|log| log.kind == JudgeLogKind::Full)
    }

    pub(crate) fn full_status(&self) -> Option<&Status> {
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
            status: invoker_api::Status {
                code: "".to_string(),
                kind: invoker_api::StatusKind::NotSet,
            },
        }
    }
}
