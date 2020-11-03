pub mod invoke;
pub mod judge_log;
pub mod valuer_proto;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum_macros::{Display, EnumString};

#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    EnumString,
    Ord,
    PartialOrd,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Hash,
)]
pub enum StatusKind {
    Queue,
    /// WA, TLE, rejected by teacher, etc
    Rejected,
    /// e.g. Coding Style Violation
    CompilationError,
    // Evaluated,
    Accepted,
    NotSet,
    InternalError,
    Skipped,
}

impl StatusKind {
    pub fn is_success(self) -> bool {
        matches!(self, Self::Accepted)
    }
}

pub mod status_codes {
    macro_rules! declare_code {
        ($code: ident) => {
            pub const $code: &str = stringify!($code);
        };

        ($code: ident, $($codes: ident),+) => {
             declare_code!($code);
             declare_code!($($codes),+);
        };
    }

    // build-related status codes
    declare_code!(
        TOOLCHAIN_SEARCH_ERROR,
        BUILT,
        COMPILATION_TIMED_OUT,
        COMPILER_FAILED
    );

    // per-test status codes
    declare_code!(
        TIME_LIMIT_EXCEEDED,
        RUNTIME_ERROR,
        TEST_PASSED,
        JUDGE_FAULT,
        WRONG_ANSWER,
        PRESENTATION_ERROR,
        LAUNCH_ERROR
    );

    // aggregated status codes
    declare_code!(ACCEPTED, PARTIAL_SOLUTION, BUILD_ERROR);
}

#[derive(Clone, Debug, Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub struct Status {
    pub kind: StatusKind,
    pub code: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JudgeRequest {
    /// Invoker will only update run, if `revision` is bigger than in DB.
    pub revision: u32,
    /// Toolchain id, for lookup in config
    pub toolchain_id: String,
    /// Problem id, for lookup in config
    pub problem_id: String,
    /// Request id (will be preserved by invoker)
    pub request_id: uuid::Uuid,
    /// Run source
    pub run_source: Vec<u8>,
}

impl std::fmt::Debug for JudgeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JudgeRequest")
            .field("revision", &self.revision)
            .field("toolchain_id", &self.toolchain_id)
            .field("problem_id", &self.problem_id)
            .field("request_id", &self.request_id)
            .field(
                "run_source",
                &format_args!("{} bytes", self.run_source.len()),
            )
            .finish()
    }
}

/// Pass this to invoker running in CLI mode
///
/// See fields' description in [JudgeRequest](JudgeRequest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliJudgeRequest {
    pub revision: u32,
    pub toolchain_id: String,
    pub problem_id: String,
    pub request_id: uuid::Uuid,
    pub run_source: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeOutcomeHeader {
    pub score: Option<u32>,
    pub status: Status,
    pub kind: valuer_proto::JudgeLogKind,
}

/// Represents Live Status Update. Some fields can be None always, or only in some updates.
#[derive(Debug, Serialize, Deserialize)]
pub struct LiveStatusUpdate {
    /// Current score. Probably, final score will be greater than or equal to `score`,
    /// but it is not guaranteed.
    pub score: Option<i32>,
    /// Id of current test (indexing starts from 1). If solution is executed on several tests, this field will contain
    /// last.
    pub current_test: Option<u32>,
}
