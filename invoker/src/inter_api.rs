use crate::judge_log;
use invoker_api::Status;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct Paths {
    /// Problem dir
    pub(crate) problem: PathBuf,
    /// Submission persistent dir
    pub(crate) submission: PathBuf,
    /// Invokation temprorary dir
    pub(crate) inv: PathBuf,
    /// Step dir
    pub(crate) step: PathBuf,
}

impl Paths {
    /// external directory child will have RW-access to
    pub(crate) fn share_dir(&self) -> PathBuf {
        self.step.join("share")
    }

    /// Root directory for child
    pub(crate) fn chroot_dir(&self) -> PathBuf {
        self.step.join("chroot")
    }
}

impl Paths {
    pub(crate) fn new(
        submission_root: &Path,
        invokation_data_root: &Path,
        step_id: u32,
        problem: &Path,
    ) -> Paths {
        let submission = submission_root.to_path_buf();
        let step = invokation_data_root.join(&format!("s-{}", step_id));
        Paths {
            submission,
            inv: invokation_data_root.to_path_buf(),
            step,
            problem: problem.to_path_buf(),
        }
    }
}

pub(crate) struct BuildRequest<'a> {
    pub(crate) paths: &'a Paths,
}

/// describes successful build outcome
pub(crate) struct Artifact {
    pub(crate) execute_command: cfg::Command,
}

pub(crate) enum BuildOutcome {
    Success(Artifact),
    Error(Status),
}

pub(crate) struct JudgeRequest<'a> {
    pub(crate) paths: &'a Paths,
    pub(crate) test_id: u32,
    pub(crate) test: &'a pom::Test,
    pub(crate) artifact: &'a Artifact,
}

#[derive(Debug, Clone)]
pub(crate) struct JudgeOutcome {
    pub(crate) status: Status,
}

pub(crate) struct ValuerNotification {
    pub(crate) test_id: u32,
    pub(crate) test_status: Status,
}

pub(crate) enum ValuerResponse {
    Test {
        test_id: u32,
    },
    Finish {
        score: u32,
        treat_as_full: bool,
        judge_log: judge_log::JudgeLog,
    },
}
