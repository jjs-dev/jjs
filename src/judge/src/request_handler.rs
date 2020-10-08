//! This module and its children are responsible for creating requests to Worker.
mod compiler;
mod exec_test;
mod transform_judge_log;
mod valuer;

use self::{
    compiler::{BuildOutcome, Compiler},
    exec_test::{exec, ExecRequest},
};
use anyhow::Context;
use judging_apis::{valuer_proto::ValuerResponse, Status};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};
use tracing::{debug, error, instrument};
/// Allows sending requests to invoker
// TODO upstream BoxedEngine to jjs-commons
pub type InvokerClient = rpc::Client<rpc::BoxEngine>;

/// Note: this is not `judging_apis::invoke::Command`, it is higher-level.
#[derive(Debug, Default)]
pub(crate) struct Command {
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: String,
}

/// Submission information, sufficient for judging
#[derive(Debug)]
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

        root.join(&short_path.path)
    }

    pub(crate) fn step_dir(&self, test_id: Option<u32>) -> PathBuf {
        match test_id {
            Some(t) => self.out_dir.join(format!("t-{}", t)),
            None => self.out_dir.join("compile"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Event {
    JudgeDone(JudgeOutcome),
    OutcomeHeader(judging_apis::JudgeOutcomeHeader),
    LiveTest(u32),
    LiveScore(u32),
}

#[derive(Debug, Clone)]
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

pub(crate) struct JudgeContext {
    /// Can be used to send requests to invoker
    pub(crate) invoker: InvokerClient,
    /// Channel that should be used for sending updates
    pub(crate) events_tx: async_channel::Sender<Event>,
}

#[instrument(skip(cx, judge_req), fields(id = %judge_req.judge_request_id))]
pub fn do_judge(mut cx: JudgeContext, judge_req: LoweredJudgeRequest) {
    debug!("Got LoweredJudgeRequest: {:?}", &judge_req);
    tokio::task::spawn(async move {
        let outcome = match cx.judge(&judge_req).await {
            Ok(o) => o,
            Err(err) => {
                error!("Invoke failed: {:#}", err);
                cx.create_fake_protocols(
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
        cx.events_tx.send(Event::JudgeDone(outcome)).await;
    });
}

impl JudgeContext {
    async fn judge(&mut self, req: &LoweredJudgeRequest) -> anyhow::Result<JudgeOutcome> {
        let compiler = Compiler { req };

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
        self.events_tx.send(Event::OutcomeHeader(header)).await.ok();
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

        let mut valuer = valuer::Valuer::new(req).context("failed to init valuer")?;
        valuer
            .write_problem_data(req)
            .await
            .context("failed to send problem data")?;
        loop {
            match valuer.poll().await? {
                ValuerResponse::Test { test_id: tid, live } => {
                    if live {
                        self.events_tx.send(Event::LiveTest(tid.get())).await;
                    }
                    let tid_u32: u32 = tid.into();
                    let test = &req.problem.tests[(tid_u32 - 1u32) as usize];
                    let exec_request = ExecRequest {
                        test,
                        test_id: tid.into(),
                    };

                    let judge_response = exec_test::exec(&req, exec_request, self)
                        .with_context(|| format!("failed to judge solution on test {}", tid))?;
                    test_results.push((tid, judge_response.clone()));
                    valuer
                        .notify_test_done(judging_apis::valuer_proto::TestDoneNotification {
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
                    self.events_tx.send(Event::LiveScore(score)).await;
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

    /// Creates `InputSource` that can be further sent to invoker
    async fn intern(&self, data: &[u8]) -> anyhow::Result<judging_apis::invoke::InputSource> {
        // TODO: optimize
        Ok(judging_apis::invoke::InputSource::Inline {
            data: data.to_vec(),
        })
    }
}
