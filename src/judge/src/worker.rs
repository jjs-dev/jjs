//! Implements Invoker Worker.
//!
//! Worker is responsible for processing `InvokeRequest`s

mod compiler;
mod exec_test;
mod invoke_util;
mod os_util;
mod transform_judge_log;
mod valuer;

use anyhow::Context;
use compiler::{BuildOutcome, Compiler};
use exec_test::{ExecRequest, TestExecutor};
use judging_apis::{
    valuer_proto::{TestDoneNotification, ValuerResponse},
    Status,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tracing::{debug, error};
use valuer::Valuer;
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Command {
    pub(crate) argv: Vec<String>,
    pub(crate) env: Vec<String>,
    pub(crate) cwd: String,
}

/// Submission information, sufficient for judging
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct LoweredJudgeRequest {
    pub(crate) compile_commands: Vec<Command>,
    pub(crate) execute_command: Command,
    pub(crate) compile_limits: pom::Limits,
    pub(crate) problem: pom::Problem,
    /// Path to problem dir
    pub(crate) problem_dir: PathBuf,
    /// Path to file containing run source
    pub(crate) run_source: PathBuf,
    /// Name of source file in sandbox. E.g., `source.cpp` for C++.
    pub(crate) source_file_name: String,
    /// Directory for emitting files (source, build, judge log)
    pub(crate) out_dir: PathBuf,
    /// Toolchain directory (i.e. sysroot for command execution)
    pub(crate) toolchain_dir: PathBuf,
    /// UUID of request
    pub(crate) judge_request_id: uuid::Uuid,
}

impl LoweredJudgeRequest {
    pub(crate) fn resolve_asset(&self, short_path: &pom::FileRef) -> PathBuf {
        let root: Cow<Path> = match short_path.root {
            pom::FileRefRoot::Problem => self.problem_dir.join("assets").into(),
            pom::FileRefRoot::Root => Path::new("/").into(),
        };

        debug!(
            "full checker path: {}",
            root.join(&short_path.path).to_str().unwrap()
        );

        root.join(&short_path.path)
    }

    pub(crate) fn step_dir(&self, test_id: Option<u32>) -> PathBuf {
        match test_id {
            Some(t) => self.out_dir.join(format!("t-{}", t)),
            None => self.out_dir.join("compile"),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) enum Request {
    Judge(LoweredJudgeRequest),
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum Response {
    JudgeDone(JudgeOutcome),
    OutcomeHeader(judging_apis::JudgeOutcomeHeader),
    LiveTest(u32),
    LiveScore(u32),
}

pub(crate) struct Worker {
    /// Minion backend to use for invocations
    minion: Arc<dyn minion::erased::Backend>,
    /// Judge configuration
    config: crate::config::JudgeConfig,
}

impl Worker {
    pub(crate) fn new(config: crate::config::JudgeConfig) -> anyhow::Result<Worker> {
        Ok(Worker {
            minion: minion::erased::setup()
                .context("minion initialization failed")?
                .into(),
            config,
        })
    }

    async fn recv(&self, stdin: &mut (impl AsyncBufReadExt + Unpin)) -> Option<Request> {
        let mut buf = String::new();
        match stdin.read_line(&mut buf).await {
            Ok(_) => {
                if buf.trim().is_empty() {
                    return None;
                }

                Some(serde_json::from_str(&buf).expect("parse error"))
            }
            Err(_) => None,
        }
    }

    async fn send(&self, resp: Response) {
        let mut stdout = tokio::io::stdout();
        let mut msg = serde_json::to_vec(&resp).expect("failed to serialize Response");
        msg.push(b'\n');
        stdout
            .write_all(&msg)
            .await
            .expect("Failed to print Response");
    }

    pub(crate) async fn main_loop(mut self) {
        let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
        while let Some(req) = self.recv(&mut stdin).await {
            match req {
                Request::Judge(judge_req) => {
                    debug!("Got LoweredJudgeRequest: {:?}", &judge_req);
                    let outcome = match self.judge(&judge_req).await {
                        Ok(o) => o,
                        Err(err) => {
                            error!("Invoke failed: {:#}", err);
                            self.create_fake_protocols(
                                &judge_req,
                                &judging_apis::Status {
                                    kind: judging_apis::StatusKind::InternalError,
                                    code: judging_apis::status_codes::JUDGE_FAULT.to_string(),
                                },
                            )
                            .await
                            .ok();
                            JudgeOutcome::Fault
                        }
                    };
                    debug!("JudgeOutcome: {:?}", &outcome);
                    self.send(Response::JudgeDone(outcome)).await;
                }
            }
        }
    }

    async fn judge(&mut self, req: &LoweredJudgeRequest) -> anyhow::Result<JudgeOutcome> {
        let compiler = Compiler {
            req,
            minion: &*self.minion,
            config: &self.config,
        };

        if !req.run_source.exists() {
            anyhow::bail!("Run source file not exists");
        }

        if !req.out_dir.exists() {
            anyhow::bail!("Run output dir not exists");
        }

        let compiler_response = compiler.compile();

        let outcome;

        match compiler_response {
            Err(err) => return Err(err).context("compilation error"),
            Ok(BuildOutcome::Error(st)) => {
                self.create_fake_protocols(req, &st).await?;
                outcome = JudgeOutcome::CompileError(st);
            }
            Ok(BuildOutcome::Success) => {
                self.run_tests(req).await.context("failed to run tests")?;

                outcome = JudgeOutcome::TestingDone;
            }
        };
        Ok(outcome)
    }

    /// Used when we are unable to produce protocols, i.e. on compilation errors
    /// and judge faults.
    async fn create_fake_protocols(
        &mut self,
        req: &LoweredJudgeRequest,
        status: &judging_apis::Status,
    ) -> anyhow::Result<()> {
        for kind in judging_apis::judge_log::JudgeLogKind::list() {
            let pseudo_valuer_proto = judging_apis::valuer_proto::JudgeLog {
                kind,
                tests: vec![],
                subtasks: vec![],
                score: 0,
                is_full: false,
            };
            let mut protocol = self.process_judge_log(&pseudo_valuer_proto, req, &[])?;
            protocol.status = status.clone();
            self.put_protocol(req, protocol).await?;
        }
        Ok(())
    }

    async fn put_outcome(
        &mut self,
        score: u32,
        status: judging_apis::Status,
        kind: judging_apis::judge_log::JudgeLogKind,
    ) {
        let header = judging_apis::JudgeOutcomeHeader {
            score: Some(score),
            status,
            kind,
        };
        self.send(Response::OutcomeHeader(header)).await;
    }

    async fn put_protocol(
        &mut self,
        req: &LoweredJudgeRequest,
        protocol: judging_apis::judge_log::JudgeLog,
    ) -> anyhow::Result<()> {
        let protocol_file_name = format!("protocol-{}.json", protocol.kind.as_str());
        let protocol_path = req.out_dir.join(protocol_file_name);
        debug!("Writing protocol to {}", protocol_path.display());
        let protocol_file = std::fs::File::create(&protocol_path)?;
        let protocol_file = std::io::BufWriter::new(protocol_file);
        serde_json::to_writer(protocol_file, &protocol)
            .context("failed to write judge log to file")?;
        self.put_outcome(protocol.score, protocol.status, protocol.kind)
            .await;
        Ok(())
    }

    async fn run_tests(&mut self, req: &LoweredJudgeRequest) -> anyhow::Result<()> {
        let mut test_results = vec![];

        let mut valuer = Valuer::new(req).context("failed to init valuer")?;
        valuer
            .write_problem_data(req)
            .await
            .context("failed to send problem data")?;
        loop {
            match valuer.poll().await? {
                ValuerResponse::Test { test_id: tid, live } => {
                    if live {
                        self.send(Response::LiveTest(tid.get())).await;
                    }
                    let tid_u32: u32 = tid.into();
                    let test = &req.problem.tests[(tid_u32 - 1u32) as usize];
                    let judge_request = ExecRequest {
                        test,
                        test_id: tid.into(),
                    };

                    let test_exec = TestExecutor {
                        exec: judge_request,
                        req,
                        minion: &*self.minion,
                        config: &self.config,
                    };

                    let judge_response = test_exec
                        .exec()
                        .with_context(|| format!("failed to judge solution on test {}", tid))?;
                    test_results.push((tid, judge_response.clone()));
                    valuer
                        .notify_test_done(TestDoneNotification {
                            test_id: tid,
                            test_status: judge_response.status,
                        })
                        .await
                        .with_context(|| {
                            format!("failed to notify valuer that test {} is done", tid)
                        })?;
                }
                ValuerResponse::Finish => {
                    break;
                }
                ValuerResponse::LiveScore { score } => {
                    self.send(Response::LiveScore(score)).await;
                }
                ValuerResponse::JudgeLog(judge_log) => {
                    let converted_judge_log = self
                        .process_judge_log(&judge_log, req, &test_results)
                        .context("failed to convert valuer judge log to invoker judge log")?;
                    self.put_protocol(req, converted_judge_log)
                        .await
                        .context("failed to save protocol")?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub(crate) enum JudgeOutcome {
    /// Compilation failed
    CompileError(Status),
    /// Run was executed on some tests successfully (i.e. without judge faults)
    /// All protocols were sent already
    TestingDone,
    /// Run was not judged, because of invocation fault
    /// Maybe, several protocols were emitted, but results are neither precise nor complete
    Fault,
}

pub async fn main() -> anyhow::Result<()> {
    let config_data = std::env::var("__JJS_WORKER_INVOKER_CONFIG")
        .context("__JJS_WORKER_INVOKER_CONFIG missing")?;
    let config = serde_json::from_str(&config_data)?;
    let w = Worker::new(config).context("worker initialization failed")?;
    w.main_loop().await;
    Ok(())
}
