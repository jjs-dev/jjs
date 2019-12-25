//! Judge log stored in FS
use invoker_api::valuer_proto::{JudgeLogKind, SubtaskId};
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
}

impl Default for JudgeLog {
    fn default() -> Self {
        Self {
            kind: JudgeLogKind::Contestant,
            tests: vec![],
            subtasks: vec![],
            compile_stdout: String::new(),
            compile_stderr: String::new(),
        }
    }
}
