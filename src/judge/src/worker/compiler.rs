use crate::worker::{invoke_util, LoweredJudgeRequest};
use anyhow::Context;
use judging_apis::{status_codes, Status, StatusKind};
use std::fs;

pub(crate) enum BuildOutcome {
    Success,
    Error(Status),
}

/// Compiler turns SubmissionInfo into Artifact
pub(crate) struct Compiler<'a> {
    pub(crate) req: &'a LoweredJudgeRequest,
    pub(crate) minion: &'a dyn minion::erased::Backend,
    pub(crate) config: &'a crate::config::JudgeConfig,
}

impl<'a> Compiler<'a> {
    pub(crate) fn compile(&self) -> anyhow::Result<BuildOutcome> {
        let sandbox = invoke_util::create_sandbox(self.req, None, self.minion, self.config)
            .context("failed to create sandbox")?;
        let step_dir = self.req.step_dir(None);
        fs::copy(
            &self.req.run_source,
            step_dir.join("data").join(&self.req.source_file_name),
        )
        .context("failed to copy source")?;

        for (i, command) in self.req.compile_commands.iter().enumerate() {
            let stdout_path = step_dir.join(&format!("stdout-{}.txt", i));
            let stderr_path = step_dir.join(&format!("stderr-{}.txt", i));

            invoke_util::log_execute_command(&command);

            let mut native_command = minion::Command::new();
            invoke_util::command_set_from_judge_req(&mut native_command, &command);
            invoke_util::command_set_stdio(&mut native_command, &stdout_path, &stderr_path);

            native_command.sandbox(sandbox.sandbox.clone());

            let child = match native_command.spawn(self.minion) {
                Ok(child) => child,
                Err(err) => {
                    let is_internal_error = match err.downcast_ref::<minion::linux::Error>() {
                        Some(e) => e.is_system(),
                        None => true,
                    };
                    if is_internal_error {
                        return Err(err.context("failed to launch child"));
                    } else {
                        return Ok(BuildOutcome::Error(Status {
                            kind: StatusKind::Rejected,
                            code: status_codes::LAUNCH_ERROR.to_string(),
                        }));
                    }
                }
            };

            let wait_result = child
                .wait_for_exit(None)
                .context("failed to wait for compiler")?;
            match wait_result {
                minion::WaitOutcome::Timeout => {
                    return Ok(BuildOutcome::Error(Status {
                        kind: StatusKind::Rejected,
                        code: status_codes::COMPILATION_TIMED_OUT.to_string(),
                    }));
                }
                minion::WaitOutcome::AlreadyFinished => unreachable!("not expected other to wait"),
                minion::WaitOutcome::Exited => {
                    if child
                        .get_exit_code()
                        .context("failed to get compiler exit code")?
                        .unwrap()
                        != 0
                    {
                        return Ok(BuildOutcome::Error(Status {
                            kind: StatusKind::Rejected,
                            code: status_codes::COMPILER_FAILED.to_string(),
                        }));
                    }
                }
            };
        }
        fs::copy(step_dir.join("data/build"), self.req.out_dir.join("build"))
            .context("failed to copy artifact to run dir")?;
        Ok(BuildOutcome::Success)
    }
}
