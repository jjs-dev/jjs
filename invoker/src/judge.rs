mod checker_proto;

use crate::{
    err,
    inter_api::{JudgeOutcome, JudgeRequest},
    invoker::{interpolate_command, InvokeContext},
    os_util, Error,
};
use invoker_api::{status_codes, Status, StatusKind};
use snafu::ResultExt;
use std::{fs, io::Write, path::PathBuf, time::Duration};

/// Runs Artifact on one test and produces output
pub(crate) struct Judge<'a> {
    pub(crate) req: JudgeRequest<'a>,
    pub(crate) ctx: InvokeContext<'a>,
}

enum RunOutcome {
    Success { out_data_path: PathBuf },
    Fail(Status),
}

impl<'a> Judge<'a> {
    fn run_solution(&self, test_data: &[u8]) -> Result<RunOutcome, Error> {
        let limits = &self.ctx.problem_cfg.limits;

        let sandbox = self.ctx.create_sandbox(limits, self.req.paths)?;

        fs::copy(
            self.req.paths.submission.join("build"),
            self.req.paths.share_dir().join("build"),
        )
        .context(err::Io {})?;

        let mut dict = self.ctx.get_common_interpolation_dict();
        dict.insert("Test.Id".to_string(), self.req.test_id.to_string().into());

        let stdout_path = self.req.paths.step.join("stdout.txt");
        let stderr_path = self.req.paths.step.join("stderr.txt");

        let command_interp = interpolate_command(&self.req.artifact.execute_command, &dict)
            .map_err(|e| err::Error::BadConfig {
                backtrace: Default::default(),
                inner: Box::new(e),
            })?;

        self.ctx.log_execute_command(&command_interp);

        let mut native_command = minion::Command::new();

        self.ctx
            .command_builder_set_from_command(&mut native_command, command_interp);
        self.ctx
            .command_builder_set_stdio(&mut native_command, &stdout_path, &stderr_path);

        native_command.dominion(sandbox);

        // capture child output
        native_command.stdin(minion::InputSpecification::Pipe);

        let mut child = match native_command.spawn(self.ctx.minion_backend) {
            Ok(child) => child,
            Err(err) => {
                if err.is_system() {
                    Err(err).context(err::Minion {})?
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
            .context(err::Minion {})?;

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
                if child.get_exit_code().context(err::Minion {})?.unwrap() != 0 {
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

    pub fn judge(&self) -> Result<JudgeOutcome, Error> {
        use std::os::unix::io::IntoRawFd;
        fs::create_dir(&self.req.paths.step).context(err::Io {})?;
        fs::create_dir(&self.req.paths.share_dir()).context(err::Io {})?;
        fs::create_dir(&self.req.paths.chroot_dir()).context(err::Io {})?;

        let input_file = self.ctx.get_asset_path(&self.req.test.path);
        let test_data = std::fs::read(input_file).expect("couldn't read test");

        let sol_file_path = match self.run_solution(&test_data)? {
            RunOutcome::Success { out_data_path } => out_data_path,
            RunOutcome::Fail(status) => return Ok(JudgeOutcome { status }),
        };
        // run checker
        let sol_file = fs::File::open(sol_file_path).unwrap();
        let sol_handle = os_util::handle_inherit(sol_file.into_raw_fd().into(), true);
        let full_checker_path = self.ctx.get_asset_path(&self.ctx.problem_data.checker_exe);
        let mut cmd = std::process::Command::new(full_checker_path);

        cmd.current_dir(self.ctx.get_problem_root());

        for arg in &self.ctx.problem_data.checker_cmd {
            cmd.arg(arg);
        }

        let test_cfg = self.req.test;

        let corr_handle = if let Some(corr_path) = &test_cfg.correct {
            let full_path = self.ctx.get_asset_path(corr_path);
            let data = fs::read(full_path).unwrap();
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
            slog::error!(self.ctx.logger, "Judge fault: checker returned non-zero");
            return return_value_for_judge_fault;
        }
        let checker_out = match String::from_utf8(os_util::handle_read_all(out_judge_side)) {
            Ok(c) => c,
            Err(_) => {
                slog::error!(self.ctx.logger, "checker produced non-utf8 output");
                return return_value_for_judge_fault;
            }
        };
        let parsed_out = match checker_proto::parse(&checker_out) {
            Ok(o) => o,
            Err(err) => {
                slog::error!(self.ctx.logger, "checker output couldn't be parsed"; "error" => ? err);
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
