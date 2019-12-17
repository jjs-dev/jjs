use invoker_api::Status;
use std::path::{Path, PathBuf};

/// This is useful abstractions, because both `Compiler` and `Judge` work with paths in similar way
#[derive(Debug, Clone)]
pub(crate) struct Paths {
    /// Problem dir
    pub(crate) problem: PathBuf,
    /// Run persistent dir
    pub(crate) run: PathBuf,
    /// Invocation temprorary dir
    pub(crate) inv: PathBuf,
    /// Step dir
    pub(crate) step: PathBuf,
}

impl Paths {
    /// External directory child will have RW-access to.
    pub(crate) fn share_dir(&self) -> PathBuf {
        self.step.join("share")
    }

    /// Root directory for child.
    pub(crate) fn chroot_dir(&self) -> PathBuf {
        self.step.join("chroot")
    }

    /// Run source
    pub(crate) fn source(&self) -> PathBuf {
        self.run.join("source")
    }

    /// Run artifact
    pub(crate) fn build(&self) -> PathBuf {
        self.run.join("build")
    }
}

impl Paths {
    pub(crate) fn new(
        run_root: &Path,
        invocation_data_root: &Path,
        step_id: u32,
        problem: &Path,
    ) -> Paths {
        let run = run_root.to_path_buf();
        let step = invocation_data_root.join(&format!("s-{}", step_id));
        Paths {
            run,
            inv: invocation_data_root.to_path_buf(),
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
