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
        match self {
            Self::Accepted => true,
            _ => false,
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeTask {
    /// Invoker will only update run, if `revision` is bigger than in DB.
    pub revision: u32,
    /// Toolchain id, for lookup in config
    pub toolchain_id: String,
    /// Problem id, for lookup in config
    pub problem_id: String,
    /// Invocation id (will be preserved by invoker)
    pub invocation_id: uuid::Uuid,
    /// Run source
    pub run_source: Vec<u8>
}

/// Pass this to invoker running in CLI mode
///
/// See fields' description in [InvokeTask](InvokeTask)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliInvokeTask {
    pub revision: u32,
    pub toolchain_id: String,
    pub problem_id: String,
    pub invocation_id: uuid::Uuid,
    pub run_source: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvokeOutcomeHeader {
    pub score: Option<u32>,
    pub status: Option<Status>,
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
