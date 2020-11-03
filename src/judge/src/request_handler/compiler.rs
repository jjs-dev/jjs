use crate::request_handler::{JudgeContext, LoweredJudgeRequest};
use anyhow::Context;
use judging_apis::{
    invoke::{Action, Command, EnvVarValue, FileId, Input, InvokeRequest, Stdio, Step},
    status_codes, Status, StatusKind,
};
use std::fs;

pub(crate) enum BuildOutcome {
    Success,
    Error(Status),
}

/// Compiler turns SubmissionInfo into Artifact
pub(crate) struct Compiler<'a> {
    pub(crate) req: &'a LoweredJudgeRequest,
    pub(crate) cx: &'a JudgeContext, // pub(crate) config: &'a crate::config::JudgeConfig,
}

const FILE_ID_SOURCE: &str = "run-source";
const FILE_ID_EMPTY: &str = "empty";

impl<'a> Compiler<'a> {
    pub(crate) async fn compile(&self) -> anyhow::Result<BuildOutcome> {
        let mut graph = InvokeRequest {
            inputs: vec![],
            outputs: vec![],
            steps: vec![],
            toolchain_dir: self.req.toolchain_dir.clone(),
        };

        let run_source = tokio::fs::read(&self.req.run_source).await?;

        graph.inputs.push(Input {
            id: FileId(FILE_ID_SOURCE.to_string()),
            source: self.cx.intern(&run_source).await?,
        });

        graph.steps.push(Step {
            stage: 0,
            action: judging_apis::invoke::Action::OpenNullFile {
                id: FileId(FILE_ID_EMPTY.to_string()),
            },
        });

        for (i, command) in self.req.compile_commands.iter().enumerate() {
            let stdout_file_id = format!("{}-stdout", i);
            let stderr_file_id = format!("{}-stderr", i);
            graph.steps.push(Step {
                stage: i as u32,
                action: judging_apis::invoke::Action::CreateFile {
                    id: FileId(stdout_file_id.clone()),
                },
            });
            graph.steps.push(Step {
                stage: i as u32,
                action: judging_apis::invoke::Action::CreateFile {
                    id: FileId(stderr_file_id.clone()),
                },
            });
            let inv_cmd = Command {
                argv: command.argv.clone(),
                env: command
                    .env
                    .clone()
                    .into_iter()
                    .map(|(k, v)| (k, EnvVarValue::Plain(v)))
                    .collect(),
                cwd: "/jjs".to_string(),
                stdio: Stdio {
                    stdin: FileId(FILE_ID_EMPTY.to_string()),
                    stdout: FileId(stdout_file_id),
                    stderr: FileId(stderr_file_id),
                },
            };

            graph.steps.push(Step {
                stage: i as u32,
                action: Action::ExecuteCommand(inv_cmd),
            });
        }
        Ok(BuildOutcome::Success)
    }
}
