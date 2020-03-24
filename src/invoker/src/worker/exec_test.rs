mod checker_proto;

use crate::worker::{invoke_util, os_util, InvokeRequest};
use anyhow::Context;
use invoker_api::{status_codes, Status, StatusKind};
use log::error;
use std::{fs, io::Write, path::PathBuf};
pub(crate) struct ExecRequest<'a> {
    pub(crate) test_id: u32,
    pub(crate) test: &'a pom::Test,
}

#[derive(Debug, Clone)]
pub(crate) struct ExecOutcome {
    pub(crate) status: Status,
}

/// Runs Artifact on one test and produces output
pub(crate) struct TestExecutor<'a> {
    pub(crate) exec: ExecRequest<'a>,
    pub(crate) req: &'a InvokeRequest,
    pub(crate) minion: &'a dyn minion::Backend,
}

enum RunOutcome {
    Success { out_data_path: PathBuf },
    Fail(Status),
}

fn map_checker_outcome_to_status(out: checker_proto::Output) -> Status {
    match out.outcome {
        checker_proto::Outcome::Ok => Status {
            kind: StatusKind::Accepted,
            code: status_codes::TEST_PASSED.to_string(),
        },
        checker_proto::Outcome::BadChecker => Status {
            kind: StatusKind::InternalError,
            code: status_codes::JUDGE_FAULT.to_string(),
        },
        checker_proto::Outcome::PresentationError => Status {
            kind: StatusKind::Rejected,
            code: status_codes::PRESENTATION_ERROR.to_string(),
        },
        checker_proto::Outcome::WrongAnswer => Status {
            kind: StatusKind::Rejected,
            code: status_codes::WRONG_ANSWER.to_string(),
        },
    }
}

impl<'a> TestExecutor<'a> {
    fn run_solution(&self, test_data: &[u8], test_id: u32) -> anyhow::Result<RunOutcome> {
        let step_dir = self.req.step_dir(Some(test_id));

        let sandbox = invoke_util::create_sandbox(self.req, Some(test_id), self.minion)?;

        fs::copy(self.req.out_dir.join("build"), step_dir.join("data/build"))
            .context("failed to copy build artifact to share dir")?;

        let stdout_path = step_dir.join("stdout.txt");
        let stderr_path = step_dir.join("stderr.txt");
        let command = &self.req.execute_command;
        invoke_util::log_execute_command(command);

        let mut native_command = minion::Command::new();

        invoke_util::command_set_from_inv_req(&mut native_command, &command);
        invoke_util::command_set_stdio(&mut native_command, &stdout_path, &stderr_path);

        native_command.dominion(sandbox.dominion.clone());

        // capture child input
        native_command.stdin(minion::InputSpecification::pipe());

        let mut child = match native_command.spawn(&*self.minion) {
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
            .wait_for_exit(None)
            .context("failed to wait for child")?;

        match wait_result {
            minion::WaitOutcome::Timeout => {
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

    pub fn exec(self) -> anyhow::Result<ExecOutcome> {
        use std::os::unix::io::IntoRawFd;
        let input_file = self.req.resolve_asset(&self.exec.test.path);
        let test_data = std::fs::read(input_file).context("failed to read test")?;

        let sol_file_path = match self.run_solution(&test_data, self.exec.test_id)? {
            RunOutcome::Success { out_data_path } => out_data_path,
            RunOutcome::Fail(status) => return Ok(ExecOutcome { status }),
        };
        // run checker
        let step_dir = self.req.step_dir(Some(self.exec.test_id));
        let sol_file = fs::File::open(sol_file_path).context("failed to open run's answer")?;
        let sol_handle = os_util::handle_inherit(sol_file.into_raw_fd().into(), true);
        let full_checker_path = self.req.resolve_asset(&self.req.problem.checker_exe);
        let mut cmd = std::process::Command::new(full_checker_path);
        cmd.current_dir(&self.req.problem_dir);

        for arg in &self.req.problem.checker_cmd {
            cmd.arg(arg);
        }

        let test_cfg = self.exec.test;

        let corr_handle = if let Some(corr_path) = &test_cfg.correct {
            let full_path = self.req.resolve_asset(corr_path);
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
        let (comments_judge_side, comments_checker_side) = os_util::make_pipe();
        cmd.env("JJS_CHECKER_COMMENT", comments_checker_side.to_string());
        let st = cmd.output().context("failed to execute checker")?;
        os_util::close(out_checker_side);
        os_util::close(comments_checker_side);
        os_util::close(corr_handle);
        os_util::close(test_handle);
        os_util::close(sol_handle);
        // TODO: capture comments
        os_util::close(comments_judge_side);

        let checker_out = std::fs::File::create(step_dir.join("check-log.txt"))?;
        let mut checker_out = std::io::BufWriter::new(checker_out);
        checker_out.write_all(b" --- stdout ---\n")?;
        checker_out.write_all(&st.stdout)?;
        checker_out.write_all(b"--- stderr ---\n")?;
        checker_out.write_all(&st.stderr)?;
        let return_value_for_judge_fault = Ok(ExecOutcome {
            status: Status {
                kind: StatusKind::InternalError,
                code: status_codes::JUDGE_FAULT.to_string(),
            },
        });

        let succ = st.status.success();
        if !succ {
            error!("Judge fault: checker returned non-zero: {}", st.status);
            os_util::close(out_judge_side);
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
                error!("checker output couldn't be parsed: {}", err);
                return return_value_for_judge_fault;
            }
        };

        let status = map_checker_outcome_to_status(parsed_out);

        Ok(ExecOutcome { status })
    }
}
