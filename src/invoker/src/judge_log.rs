use anyhow::{bail, Context};
use bitflags::bitflags;
use serde::Serialize;

bitflags! {
    pub(crate) struct VisibleComponents: u32 {
        const TEST_DATA = 1;
        /// Solution stdout & stderr
        const OUTPUT = 2;
        /// Test answer
        const ANSWER = 4;
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
    pub(crate) test_answer: Option<String>,
    #[serde(skip)]
    pub(crate) components: VisibleComponents,
}

impl std::str::FromStr for JudgeLogRow {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<JudgeLogRow, Self::Err> {
        let parts = s.split_ascii_whitespace();
        let parts = parts.collect::<Vec<_>>();
        if parts.len() != 5 {
            bail!("invalid item count: expected 5, got {}", parts.len());
        }
        let test_id = parts[0].parse().context("test_id is not non-zero int")?;
        let test_id = pom::TestId(test_id);
        let status_kind: invoker_api::StatusKind = parts[1]
            .parse()
            .with_context(|| format!("invalid status kind {}", &parts[1]))?;

        let status_code = parts[2].to_string();

        let score = parts[3].parse().context("score is not u32")?;

        let flags: u32 = parts[4].parse().context("flags is not u32")?;

        let flags = crate::judge_log::VisibleComponents::from_bits(flags).with_context(|| {
            format!(
                "invalid components visibility flags: available {}, got {}",
                crate::judge_log::VisibleComponents::all().bits(),
                flags
            )
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
            test_answer: None,
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
