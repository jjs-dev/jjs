use crate::{
    inter_api::{Artifact, BuildOutcome, BuildRequest},
    invoke_util,
    invoker::{interpolate_command, InvokeContext},
};
use anyhow::Context;
use invoker_api::{status_codes, Status, StatusKind};
use std::{fs, time::Duration};

/// Compiler turns SubmissionInfo into Artifact
pub(crate) struct Compiler<'a> {
    pub(crate) ctx: &'a dyn InvokeContext,
}

impl<'a> Compiler<'a> {
    pub(crate) fn compile(&self, req: BuildRequest) -> anyhow::Result<BuildOutcome> {
        fs::create_dir(&req.paths.step).context("failed to create compile step dir")?;
        fs::create_dir(req.paths.chroot_dir()).context("failed to create compile chroot dir")?;
        fs::create_dir(req.paths.share_dir()).context("failed to create compile share dir")?;

        let toolchain = self.ctx.env().toolchain_cfg;
        let limits = &toolchain.limits;

        let sandbox = crate::invoke_util::create_sandbox(
            self.ctx.env().cfg,
            limits,
            req.paths,
            self.ctx.env().minion_backend,
        )
        .context("failed to create sandbox")?;

        fs::copy(
            req.paths.source(),
            req.paths.share_dir().join(&toolchain.filename),
        )
        .context("failed to copy source")?;

        for (i, command_template) in toolchain.build_commands.iter().enumerate() {
            let dict = invoke_util::get_common_interpolation_dict(
                self.ctx.env().run_props,
                self.ctx.env().toolchain_cfg,
            );
            let stdout_path = req.paths.step.join(&format!("stdout-{}.txt", i));
            let stderr_path = req.paths.step.join(&format!("stderr-{}.txt", i));

            let command_interp = interpolate_command(command_template, &dict)
                .context("invalid compiler command template")?;

            invoke_util::log_execute_command(&command_interp);

            let mut native_command = minion::Command::new();
            invoke_util::command_set_from_interp(&mut native_command, &command_interp);
            invoke_util::command_set_stdio(&mut native_command, &stdout_path, &stderr_path);

            native_command.dominion(sandbox.clone());

            let child = match native_command.spawn(self.ctx.env().minion_backend) {
                Ok(child) => child,
                Err(err) => {
                    if err.is_system() {
                        return Err(anyhow::Error::new(err).context("failed to launch child"));
                    } else {
                        return Ok(BuildOutcome::Error(Status {
                            kind: StatusKind::Rejected,
                            code: status_codes::LAUNCH_ERROR.to_string(),
                        }));
                    }
                }
            };

            let wait_result = child
                .wait_for_exit(Duration::from_millis(limits.time))
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
        fs::copy(req.paths.share_dir().join("build"), req.paths.build())
            .context("failed to copy artifact to run dir")?;
        Ok(BuildOutcome::Success(Artifact {
            execute_command: toolchain.run_command.clone(),
        }))
    }
}
