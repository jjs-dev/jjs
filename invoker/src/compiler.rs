use crate::{
    err,
    inter_api::{Artifact, BuildOutcome, BuildRequest},
    invoker::{interpolate_command, InvokeContext},
    Error,
};
use invoker_api::{status_codes, Status, StatusKind};
use snafu::ResultExt;
use std::{fs, time::Duration};

/// Compiler turns SubmissionInfo into Artifact
pub(crate) struct Compiler<'a> {
    pub(crate) ctx: InvokeContext<'a>,
}

impl<'a> Compiler<'a> {
    pub(crate) fn compile(&self, req: BuildRequest) -> Result<BuildOutcome, Error> {
        fs::create_dir(&req.paths.step).context(err::Io {})?;
        fs::create_dir(req.paths.chroot_dir()).context(err::Io {})?;
        fs::create_dir(req.paths.share_dir()).context(err::Io {})?;

        let limits = &self.ctx.toolchain_cfg.limits;

        let toolchain = &self.ctx.toolchain_cfg;

        let sandbox = self.ctx.create_sandbox(limits, req.paths)?;

        fs::copy(
            req.paths.submission.join("source"),
            req.paths.share_dir().join(&toolchain.filename),
        )
        .context(err::Io {})?;

        for (i, command_template) in toolchain.build_commands.iter().enumerate() {
            let dict = self.ctx.get_common_interpolation_dict();
            let stdout_path = req.paths.step.join(&format!("stdout-{}.txt", i));
            let stderr_path = req.paths.step.join(&format!("stderr-{}.txt", i));

            let command_interp = interpolate_command(command_template, &dict).map_err(|e| {
                err::Error::BadConfig {
                    backtrace: Default::default(),
                    inner: Box::new(e),
                }
            })?;

            self.ctx.log_execute_command(&command_interp);

            let mut native_command = minion::Command::new();
            self.ctx
                .command_builder_set_from_command(&mut native_command, command_interp);
            self.ctx
                .command_builder_set_stdio(&mut native_command, &stdout_path, &stderr_path);

            native_command.dominion(sandbox.clone());

            let mut child = match native_command.spawn(self.ctx.minion_backend) {
                Ok(child) => child,
                Err(err) => {
                    if err.is_system() {
                        Err(err).context(err::Minion {})?
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
                .context(err::Minion {})?;
            match wait_result {
                minion::WaitOutcome::Timeout => {
                    child.kill().ok(); //.ok() to ignore: kill on best effort basis
                    return Ok(BuildOutcome::Error(Status {
                        kind: StatusKind::Rejected,
                        code: status_codes::COMPILATION_TIMED_OUT.to_string(),
                    }));
                }
                minion::WaitOutcome::AlreadyFinished => unreachable!("not expected other to wait"),
                minion::WaitOutcome::Exited => {
                    if child.get_exit_code().context(err::Minion {})?.unwrap() != 0 {
                        return Ok(BuildOutcome::Error(Status {
                            kind: StatusKind::Rejected,
                            code: status_codes::COMPILER_FAILED.to_string(),
                        }));
                    }
                }
            };
        }
        fs::copy(
            req.paths.share_dir().join("build"),
            req.paths.submission.join("build"),
        )
        .context(err::Io {})?;
        Ok(BuildOutcome::Success(Artifact {
            execute_command: toolchain.run_command.clone(),
        }))
    }
}
