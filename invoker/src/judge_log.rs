use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct JudgeLogRow {
    pub(crate) test_id: u32,
    pub(crate) status_code: String,
    pub(crate) status_kind: invoker_api::StatusKind,
    pub(crate) score: u32,
}

#[derive(Debug, Snafu)]
pub(crate) enum ParseRowError {
    #[snafu(display("expected 4 elements, got {}", actual))]
    ElementCountMismatch { actual: usize },
    #[snafu(display("failed to parse status kind '{}': {}", actual, source))]
    InvalidStatusKind {
        source: <invoker_api::StatusKind as std::str::FromStr>::Err,
        actual: String,
    },
    #[snafu(display("failed to parse numeric field: {}", source))]
    InvalidIntField { source: std::num::ParseIntError },
}

impl std::str::FromStr for JudgeLogRow {
    type Err = ParseRowError;

    fn from_str(s: &str) -> Result<JudgeLogRow, Self::Err> {
        let parts = s.split_ascii_whitespace();
        let parts = parts.collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(ParseRowError::ElementCountMismatch {
                actual: parts.len(),
            });
        }
        let test_id = parts[0].parse().context(InvalidIntField {})?;
        let status_kind: invoker_api::StatusKind = parts[1].parse().context(InvalidStatusKind {
            actual: parts[1].to_string(),
        })?;

        let status_code = parts[2].to_string();

        let score = parts[3].parse().context(InvalidIntField {})?;

        let jr = JudgeLogRow {
            test_id,
            status_code,
            status_kind,
            score,
        };

        Ok(jr)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct JudgeLog {
    pub(crate) name: String,
    pub(crate) tests: Vec<JudgeLogRow>,
    pub(crate) compile_stdout: String,
    pub(crate) compile_stderr: String,
}
