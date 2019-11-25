mod checker_proto;

use crate::{
    inter_api::{JudgeOutcome, JudgeRequest},
    invoke_util,
    invoker::{interpolate_command, InvokeContext},
    os_util,
};
use anyhow::Context;
use invoker_api::{status_codes, Status, StatusKind};
use slog_scope::error;
use std::{fs, path::PathBuf, time::Duration};

/// Runs Artifact on one test and produces output
pub(crate) struct Judge<'a> {
    pub(crate) req: JudgeRequest<'a>,
    pub(crate) ctx: &'a dyn InvokeContext,
}

enum RunOutcome {
    Success { out_data_path: PathBuf },
    Fail(Status),
}

impl<'a> Judge<'a> {
    fn run_solution(&self, test_data: &[u8]) -> anyhow::Result<RunOutcome> {
        let limits = &self.ctx.env().problem_cfg.limits;

        let sandbox = invoke_util::create_sandbox(
            self.ctx.env().cfg,
            limits,
            self.req.paths,
            self.ctx.env().minion_backend,
        )?;

        fs::copy(
            self.req.paths.build(),
            self.req.paths.share_dir().join("build"),
        )
        .context("failed to copy build artifact to share dir")?;

        let mut dict = invoke_util::get_common_interpolation_dict(
            self.ctx.env().run_props,
            self.ctx.env().toolchain_cfg,
        );
        dict.insert("Test.Id".to_string(), self.req.test_id.to_string().into());

        let stdout_path = self.req.paths.step.join("stdout.txt");
        let stderr_path = self.req.paths.step.join("stderr.txt");

        let command_interp = interpolate_command(&self.req.artifact.execute_command, &dict)
            .map_err(|e| {
                anyhow::Error::new(e).context("Config specifies incorrect execute command")
            })?;

        invoke_util::log_execute_command(&command_interp);

        let mut native_command = minion::Command::new();

        invoke_util::command_set_from_interp(&mut native_command, &command_interp);
        invoke_util::command_set_stdio(&mut native_command, &stdout_path, &stderr_path);

        native_command.dominion(sandbox);

        // capture child input
        native_command.stdin(minion::InputSpecification::pipe());

        let mut child = match native_command.spawn(self.ctx.env().minion_backend) {
            Ok(child) => child,
            Err(err) => {
                if err.is_system() {
                    Err(err).context("failed to spawn solution")?
                } else {
                    return Ok(RunOutcome::Fail(Status {
                        kind: StatusKind::Rejected,
                        code: status_codes::LAUNCH_ERROR.to_string(),
                    }));
                }
            }
        };
        let mut stdin = child.stdin().unwrap();
        stdin.write_all(test_data).ok();
        std::mem::drop(stdin); // close pipe

        let wait_result = child
            .wait_for_exit(Duration::from_millis(limits.time))
            .context("failed to wait for child")?;

        match wait_result {
            minion::WaitOutcome::Timeout => {
                child.kill().ok();
                return Ok(RunOutcome::Fail(Status {
                    kind: StatusKind::Rejected,
                    code: status_codes::TIME_LIMIT_EXCEEDED.to_string(),
                }));
            }
            minion::WaitOutcome::AlreadyFinished => unreachable!("not expected other to wait"),
            minion::WaitOutcome::Exited => {
                if child
                    .get_exit_code()
                    .context("failed to get exit code")?
                    .unwrap()
                    != 0
                {
                    return Ok(RunOutcome::Fail(Status {
                        kind: StatusKind::Rejected,
                        code: status_codes::RUNTIME_ERROR.to_string(),
                    }));
                }
            }
        }

        Ok(RunOutcome::Success {
            out_data_path: stdout_path,
        })
    }

    pub fn judge(&self) -> anyhow::Result<JudgeOutcome> {
        use std::os::unix::io::IntoRawFd;
        fs::create_dir(&self.req.paths.step).context("failed to create step dir")?;
        fs::create_dir(&self.req.paths.share_dir()).context("failed to create share dir")?;
        fs::create_dir(&self.req.paths.chroot_dir()).context("failed to create chroot dir")?;

        let input_file = self.ctx.resolve_asset(&self.req.test.path);
        let test_data = std::fs::read(input_file).context("failed to read test")?;

        let sol_file_path = match self.run_solution(&test_data)? {
            RunOutcome::Success { out_data_path } => out_data_path,
            RunOutcome::Fail(status) => return Ok(JudgeOutcome { status }),
        };
        // run checker
        let sol_file = fs::File::open(sol_file_path).context("failed to open run's answer")?;
        let sol_handle = os_util::handle_inherit(sol_file.into_raw_fd().into(), true);
        let full_checker_path = self
            .ctx
            .resolve_asset(&self.ctx.env().problem_data.checker_exe);
        let mut cmd = std::process::Command::new(full_checker_path);

        cmd.current_dir(self.ctx.env().problem_root());

        for arg in &self.ctx.env().problem_data.checker_cmd {
            cmd.arg(arg);
        }

        let test_cfg = self.req.test;

        let corr_handle = if let Some(corr_path) = &test_cfg.correct {
            let full_path = self.ctx.resolve_asset(corr_path);
            let data = fs::read(full_path).context("failed to read correct answer")?;
            os_util::buffer_to_file(&data, "invoker-correct-data")
        } else {
            os_util::buffer_to_file(&[], "invoker-correct-data")
        };
        let test_handle = os_util::buffer_to_file(&test_data, "invoker-test-data");

        cmd.env("JJS_CORR", corr_handle.to_string());
        cmd.env("JJS_SOL", sol_handle.to_string());
        cmd.env("JJS_TEST", test_handle.to_string());

        let (out_judge_side, out_checker_side) = os_util::make_pipe();
        cmd.env("JJS_CHECKER_OUT", out_checker_side.to_string());
        let (_comments_judge_side, comments_checker_side) = os_util::make_pipe();
        cmd.env("JJS_CHECKER_COMMENT", comments_checker_side.to_string());
        let st = cmd.status().map(|st| st.success());
        os_util::close(out_checker_side);
        os_util::close(comments_checker_side);
        os_util::close(corr_handle);
        os_util::close(test_handle);

        let return_value_for_judge_fault = Ok(JudgeOutcome {
            status: Status {
                kind: StatusKind::InternalError,
                code: status_codes::JUDGE_FAULT.to_string(),
            },
        });

        let st = st.unwrap_or(false);
        if !st {
            error!("Judge fault: checker returned non-zero");
            return return_value_for_judge_fault;
        }
        let checker_out = match String::from_utf8(os_util::handle_read_all(out_judge_side)) {
            Ok(c) => c,
            Err(_) => {
                error!("checker produced non-utf8 output");
                return return_value_for_judge_fault;
            }
        };
        let parsed_out = match checker_proto::parse(&checker_out) {
            Ok(o) => o,
            Err(err) => {
                error!( "checker output couldn't be parsed"; "error" => ? err);
                return return_value_for_judge_fault;
            }
        };

        let outcome = match parsed_out.outcome {
            checker_proto::Outcome::Ok => JudgeOutcome {
                status: Status {
                    kind: StatusKind::Accepted,
                    code: status_codes::TEST_PASSED.to_string(),
                },
            },
            checker_proto::Outcome::BadChecker => JudgeOutcome {
                status: Status {
                    kind: StatusKind::InternalError,
                    code: status_codes::JUDGE_FAULT.to_string(),
                },
            },
            checker_proto::Outcome::PresentationError => JudgeOutcome {
                status: Status {
                    kind: StatusKind::Rejected,
                    code: status_codes::PRESENTATION_ERROR.to_string(),
                },
            },
            checker_proto::Outcome::WrongAnswer => JudgeOutcome {
                status: Status {
                    kind: StatusKind::Rejected,
                    code: status_codes::WRONG_ANSWER.to_string(),
                },
            },
        };

        Ok(outcome)
    }
}
