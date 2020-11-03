mod checker_proto;

use super::{JudgeContext, LoweredJudgeRequest};
use anyhow::Context;
use judging_apis::{
    invoke::{Action, Command, EnvVarValue, Expose, FileId, Input, InputSource, Stdio, Step},
    status_codes, Status, StatusKind,
};
use std::{io::Write, path::PathBuf};
use tracing::{debug, error};
pub(crate) struct ExecRequest<'a> {
    pub(crate) test_id: u32,
    pub(crate) test: &'a pom::Test,
}

#[derive(Debug, Clone)]
pub(crate) struct ExecOutcome {
    pub(crate) status: Status,
    pub(crate) resource_usage: minion::ResourceUsageData,
}

enum RunOutcomeVar {
    Success { out_data_path: PathBuf },
    Fail(Status),
}

struct RunOutcome {
    var: RunOutcomeVar,
    resource_usage: minion::ResourceUsageData,
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

/// Runs Artifact on one test and produces output
pub async fn exec(
    judge_req: &LoweredJudgeRequest,
    exec_req: ExecRequest<'_>,
    cx: &JudgeContext,
) -> anyhow::Result<ExecOutcome> {
    let mut invoke_request = judging_apis::invoke::InvokeRequest {
        steps: vec![],
        inputs: vec![],
        outputs: vec![],
        toolchain_dir: judge_req.toolchain_dir.clone(),
    };
    let input_file = judge_req.resolve_asset(&exec_req.test.path);
    let test_data = std::fs::read(input_file).context("failed to read test")?;
    const PREPARE_STAGE: u32 = 0;
    const EXEC_SOLUTION_STAGE: u32 = 1;
    const TEST_DATA_INPUT_FILE: &str = "test-data";
    const EXEC_SOLUTION_OUTPUT_FILE: &str = "solution-output";
    const EXEC_SOLUTION_ERROR_FILE: &str = "solution-error";
    const CORRECT_ANSWER_FILE: &str = "correct";
    const EMPTY_FILE: &str = "empty";

    const EXEC_CHECKER_STAGE: u32 = 2;
    // create an input with the test data
    {
        let test_data_input = Input {
            id: FileId(TEST_DATA_INPUT_FILE.to_string()),
            source: cx.intern(&test_data).await?,
        };
        invoke_request.inputs.push(test_data_input);
    }
    // prepare empty input
    {
        invoke_request.steps.push(Step {
            stage: PREPARE_STAGE,
            action: Action::OpenNullFile {
                id: FileId(EMPTY_FILE.to_string()),
            },
        });
    }
    // prepare files for stdout & stderr
    {
        invoke_request.steps.push(Step {
            stage: EXEC_SOLUTION_STAGE,
            action: Action::CreateFile {
                id: FileId(EXEC_SOLUTION_OUTPUT_FILE.to_string()),
            },
        });
        invoke_request.steps.push(Step {
            stage: EXEC_SOLUTION_STAGE,
            action: Action::CreateFile {
                id: FileId(EXEC_SOLUTION_ERROR_FILE.to_string()),
            },
        })
    }
    // produce a step for executing solution
    {
        let exec_solution_step = Step {
            stage: EXEC_SOLUTION_STAGE,
            action: Action::ExecuteCommand(Command {
                argv: judge_req.execute_command.argv.clone(),
                env: judge_req
                    .execute_command
                    .env
                    .iter()
                    .cloned()
                    .map(|(k, v)| (k, EnvVarValue::Plain(v)))
                    .collect(),
                cwd: judge_req.execute_command.cwd.clone(),
                stdio: Stdio {
                    stdin: FileId(TEST_DATA_INPUT_FILE.to_string()),
                    stdout: FileId(EXEC_SOLUTION_OUTPUT_FILE.to_string()),
                    stderr: FileId(EXEC_SOLUTION_ERROR_FILE.to_string()),
                },
                expose: Vec::new(),
            }),
        };
        invoke_request.steps.push(exec_solution_step);
    }
    // provide a correct answer if requested
    {
        let source = if let Some(corr_path) = &exec_req.test.correct {
            let full_path = judge_req.resolve_asset(corr_path);
            let data = tokio::fs::read(full_path)
                .await
                .context("failed to read correct answer")?;
            cx.intern(&data).await?
        } else {
            cx.intern(&[]).await?
        };
    }
    // generate checker feedback files
    const CHECKER_DECISION: &str = "checker-decision";
    const CHECKER_COMMENTS: &str = "checker-comment";
    {
        invoke_request.steps.push(Step {
            stage: EXEC_CHECKER_STAGE,
            action: Action::CreateFile {
                id: FileId(CHECKER_DECISION.to_string()),
            },
        });
        invoke_request.steps.push(Step {
            stage: EXEC_CHECKER_STAGE,
            action: Action::CreateFile {
                id: FileId(CHECKER_COMMENTS.to_string()),
            },
        })
    }
    // produce a step for executing checker
    {
        let exec_checker_step = Step {
            stage: EXEC_CHECKER_STAGE,
            action: Action::ExecuteCommand(Command {
                argv: judge_req.problem.checker_cmd.clone(),
                env: vec![
                    (
                        "JJS_CORR".to_string(),
                        EnvVarValue::File(FileId(CORRECT_ANSWER_FILE.to_string())),
                    ),
                    (
                        "JJS_SOL".to_string(),
                        EnvVarValue::File(FileId(EXEC_SOLUTION_OUTPUT_FILE.to_string())),
                    ),
                    (
                        "JJS_TEST".to_string(),
                        EnvVarValue::File(FileId(TEST_DATA_INPUT_FILE.to_string())),
                    ),
                    (
                        "JJS_CHECKER_OUT".to_string(),
                        EnvVarValue::File(FileId(CHECKER_DECISION.to_string())),
                    ),
                    (
                        "JJS_CHECKER_COMMENT".to_string(),
                        EnvVarValue::File(FileId(CHECKER_COMMENTS.to_string())),
                    ),
                ]
                .into_iter()
                .collect(),
                cwd: "/".to_string(),
                stdio: Stdio {
                    stdin: FileId(EMPTY_FILE.to_string()),
                    stdout: FileId(EMPTY_FILE.to_string()),
                    stderr: FileId(EMPTY_FILE.to_string()),
                },
                expose: vec![Expose::Problem],
            }),
        };

        invoke_request.steps.push(exec_checker_step);
    }

    let st = cmd.output().context("failed to execute checker")?;

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
        resource_usage: Default::default(),
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

    Ok(ExecOutcome {
        status,
        resource_usage: run_outcome.resource_usage,
    })
}
