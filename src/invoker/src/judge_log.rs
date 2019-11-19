use anyhow::{bail, Context};
use bitflags::bitflags;
use serde::Serialize;
use std::str::FromStr;

bitflags! {
    pub(crate) struct TestVisibleComponents: u32 {
        /// Test input data
        const TEST_DATA = 1;
        /// Solution stdout & stderr
        const OUTPUT = 2;
        /// Test answer
        const ANSWER = 4;
    }
}

bitflags! {
    pub(crate) struct SubtaskVisibleComponents: u32 {
        /// Score gained for this subtask
        const SCORE = 1;
    }
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct JudgeLogTestRow {
    pub(crate) test_id: pom::TestId,
    pub(crate) status_code: String,
    pub(crate) status_kind: invoker_api::StatusKind,
    pub(crate) test_stdin: Option<String>,
    pub(crate) test_stdout: Option<String>,
    pub(crate) test_stderr: Option<String>,
    pub(crate) test_answer: Option<String>,
    #[serde(skip)]
    pub(crate) components: TestVisibleComponents,
}

impl FromStr for JudgeLogTestRow {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<JudgeLogTestRow, Self::Err> {
        let parts = s.split_ascii_whitespace();
        let parts = parts.collect::<Vec<_>>();
        if parts.len() != 4 {
            bail!("invalid item count: expected 4, got {}", parts.len());
        }
        let test_id = parts[0].parse().context("test_id is not non-zero int")?;
        let test_id = pom::TestId(test_id);
        let status_kind: invoker_api::StatusKind = parts[1]
            .parse()
            .with_context(|| format!("invalid status kind {}", &parts[1]))?;

        let status_code = parts[2].to_string();

        let flags: u32 = parts[3].parse().context("flags is not u32")?;

        let flags =
            crate::judge_log::TestVisibleComponents::from_bits(flags).with_context(|| {
                format!(
                    "invalid components visibility flags: available {}, got {}",
                    crate::judge_log::TestVisibleComponents::all().bits(),
                    flags
                )
            })?;
        let jr = JudgeLogTestRow {
            test_id,
            status_code,
            status_kind,
            components: flags,
            test_stdin: None,
            test_stdout: None,
            test_stderr: None,
            test_answer: None,
        };

        Ok(jr)
    }
}

#[derive(Debug, Serialize, Copy, Clone)]
pub(crate) struct SubtaskId(std::num::NonZeroU32);

#[derive(Debug, Serialize, Clone)]
pub(crate) struct JudgeLogSubtaskRow {
    pub(crate) subtask_id: SubtaskId,
    pub(crate) score: Option<u32>,
    #[serde(skip)]
    pub(crate) components: SubtaskVisibleComponents,
}

impl FromStr for JudgeLogSubtaskRow {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let parts = s.split_ascii_whitespace();
        let parts: Vec<_> = parts.collect();
        if parts.len() != 3 {
            bail!("invalid item count: expected 3, got {}", parts.len());
        }

        let subtask_id = parts[0].parse().context("failed to parse subtask id")?;
        let score = parts[1].parse().context("failed to parse subtask score")?;
        let components_bits = parts[2]
            .parse()
            .context("failed to parse visible components")?;
        let components =
            SubtaskVisibleComponents::from_bits(components_bits).with_context(|| {
                format!(
                    "invalid subtask components visibility flags: available {}, got {}",
                    SubtaskVisibleComponents::all().bits(),
                    components_bits
                )
            })?;
        let score = if components.contains(SubtaskVisibleComponents::SCORE) {
            Some(score)
        } else {
            None
        };
        let row = JudgeLogSubtaskRow {
            subtask_id: SubtaskId(subtask_id),
            score,
            components,
        };
        Ok(row)
    }
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct JudgeLog {
    pub(crate) name: String,
    pub(crate) tests: Vec<JudgeLogTestRow>,
    pub(crate) subtasks: Vec<JudgeLogSubtaskRow>,
    pub(crate) compile_stdout: String,
    pub(crate) compile_stderr: String,
}
