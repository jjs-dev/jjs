use bitflags::bitflags;
use serde::Serialize;
use snafu::{ResultExt, Snafu};

bitflags! {
    pub(crate) struct VisibleComponents: u32 {
        const TEST_DATA = 1;
        /// solution stdout & stderr
        const OUTPUT = 2;
    }
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct JudgeLogRow {
    pub(crate) test_id: pom::TestId,
    pub(crate) status_code: String,
    pub(crate) status_kind: invoker_api::StatusKind,
    pub(crate) score: u32,
    pub(crate) test_stdin: Option<String>,
    pub(crate) test_stdout: Option<String>,
    pub(crate) test_stderr: Option<String>,
    #[serde(skip)]
    pub(crate) components: VisibleComponents,
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
    #[snafu(display(
        "Invalid component visibility flags specified: got {}, but flags must be subset of {}",
        got,
        available
    ))]
    InvalidComponentsSpecification { got: u32, available: u32 },
}

impl std::str::FromStr for JudgeLogRow {
    type Err = ParseRowError;

    fn from_str(s: &str) -> Result<JudgeLogRow, Self::Err> {
        let parts = s.split_ascii_whitespace();
        let parts = parts.collect::<Vec<_>>();
        if parts.len() != 5 {
            return Err(ParseRowError::ElementCountMismatch {
                actual: parts.len(),
            });
        }
        let test_id = parts[0].parse().context(InvalidIntField {})?;
        let test_id = pom::TestId(test_id);
        let status_kind: invoker_api::StatusKind = parts[1].parse().context(InvalidStatusKind {
            actual: parts[1].to_string(),
        })?;

        let status_code = parts[2].to_string();

        let score = parts[3].parse().context(InvalidIntField {})?;

        let flags: u32 = parts[4].parse().context(InvalidIntField {})?;

        let flags = crate::judge_log::VisibleComponents::from_bits(flags)
            .ok_or(())
            .map_err(|_| ParseRowError::InvalidComponentsSpecification {
                got: flags,
                available: crate::judge_log::VisibleComponents::all().bits(),
            })?;
        let jr = JudgeLogRow {
            test_id,
            status_code,
            status_kind,
            score,
            components: flags,
            test_stdin: None,
            test_stdout: None,
            test_stderr: None,
        };

        Ok(jr)
    }
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct JudgeLog {
    pub(crate) name: String,
    pub(crate) tests: Vec<JudgeLogRow>,
    pub(crate) compile_stdout: String,
    pub(crate) compile_stderr: String,
}
