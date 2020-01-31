mod compiler;
mod exec_test;
mod invoke_util;
pub(crate) mod judge_log;
mod os_util;
mod transform_judge_log;
mod valuer;

use anyhow::Context;
use cfg::Limits;
use compiler::{BuildOutcome, Compiler};
use crossbeam_channel::{Receiver, Sender};
use exec_test::{ExecRequest, TestExecutor};
use invoker_api::{
    valuer_proto::{TestDoneNotification, ValuerResponse},
    Status,
};
use judge_log::JudgeLogs;
use serde::Serialize;
use slog_scope::{debug, error};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};
use valuer::Valuer;
#[derive(Default, Debug, Clone)]
pub(crate) struct Command {
    pub(crate) argv: Vec<String>,
    pub(crate) env: Vec<String>,
    pub(crate) cwd: String,
}

/// Submission information, sufficient for judging
#[derive(Clone, Debug)]
pub(crate) struct InvokeRequest {
    pub(crate) compile_commands: Vec<Command>,
    pub(crate) execute_command: Command,
    pub(crate) compile_limits: Limits,
    pub(crate) execute_limits: Limits,
    pub(crate) problem_data: pom::Problem,
    /// Path to problem dir
    pub(crate) problem_dir: PathBuf,
    /// Path to file containing run source
    pub(crate) run_source: PathBuf,
    /// Name of source file in sandbox. E.g., `source.cpp` for C++.
    pub(crate) source_file_name: String,
    /// Directory for emitting files (source, build, judge log)
    pub(crate) out_dir: PathBuf,
    /// Dir with global files, like standard checkers
    pub(crate) global_dir: PathBuf,
    /// Toolchains dir
    pub(crate) toolchains_dir: PathBuf,
    /// UUID of request
    pub(crate) invocation_id: uuid::Uuid,
    /// Minion backend to use for invocations
    pub(crate) minion: Arc<dyn minion::Backend>,
}

impl InvokeRequest {
    pub(crate) fn resolve_asset(&self, short_path: &pom::FileRef) -> PathBuf {
        let root: Cow<Path> = match short_path.root {
            pom::FileRefRoot::Problem => self.problem_dir.join("assets").into(),
            pom::FileRefRoot::System => self.global_dir.clone().into(),
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

pub(crate) enum Request {
    Invoke(InvokeRequest),
}

#[derive(Debug)]
pub(crate) enum Response {
    Invoke(InvokeOutcome),
    LiveTest(u32),
    LiveScore(u32),
}

pub(crate) struct Worker {
    sender: Sender<Response>,
    receiver: Receiver<Request>,
}

impl Worker {
    pub(crate) fn new(sender: Sender<Response>, receiver: Receiver<Request>) -> Worker {
        Worker { sender, receiver }
    }

    fn self_isolate(&mut self) {
        #[cfg(target_os = "linux")]
        {
            nix::sched::unshare(
                nix::sched::CloneFlags::CLONE_NEWNET | nix::sched::CloneFlags::CLONE_FILES,
            )
            .expect("failed to unshare");
        }
    }

    pub(crate) fn main_loop(mut self) {
        self.self_isolate();
        while let Ok(req) = self.receiver.recv() {
            match req {
                Request::Invoke(inv_req) => {
                    debug!("Got InvokeRequest: {:?}", &inv_req);
                    let outcome = self.invoke(&inv_req).unwrap_or_else(|err| {
                        error!("Invoke failed: {}", err);
                        InvokeOutcome::Fault
                    });
                    debug!("InvokeOutcome: {:?}", &outcome);
                    self.sender
                        .send(Response::Invoke(outcome))
                        .expect("failed to send InvokeOutcome");
                }
            }
        }
    }

    fn invoke(&mut self, req: &InvokeRequest) -> anyhow::Result<InvokeOutcome> {
        let compiler = Compiler { req };

        if !req.run_source.exists() {
            anyhow::bail!("Run source file not exisis");
        }

        if !req.out_dir.exists() {
            anyhow::bail!("Run output dir not exists");
        }

        let compiler_response = compiler.compile();

        let outcome;

        match compiler_response {
            Err(err) => return Err(err),
            Ok(BuildOutcome::Error(st)) => {
                outcome = InvokeOutcome::CompileError(st);
            }
            Ok(BuildOutcome::Success) => {
                let judge_logs = self.run_tests(req)?;

                outcome = InvokeOutcome::Judge(judge_logs);
            }
        };
        if let InvokeOutcome::Judge(judge_logs) = &outcome {
            let judge_log_path = req.out_dir.join("log.json");
            debug!("Writing judging log to {}", judge_log_path.display());
            let judge_log_file = std::fs::File::create(&judge_log_path)?;
            let judge_log_file = std::io::BufWriter::new(judge_log_file);
            serde_json::to_writer(judge_log_file, &judge_logs.0)
                .context("failed to write judge log to file")?;
            debug!("Invokation finished"; "status" => ?outcome.status());
        }
        Ok(outcome)
    }

    fn run_tests(&mut self, req: &InvokeRequest) -> anyhow::Result<JudgeLogs> {
        let mut test_results = vec![];

        let mut valuer = Valuer::new(req).context("failed to init valuer")?;
        valuer
            .write_problem_data(req)
            .context("failed to send problem data")?;
        let mut judge_logs = Vec::new();
        loop {
            match valuer.poll()? {
                ValuerResponse::Test { test_id: tid, live } => {
                    if live {
                        self.sender.send(Response::LiveTest(tid.get())).ok();
                    }
                    let tid_u32: u32 = tid.into();
                    let test = &req.problem_data.tests[(tid_u32 - 1u32) as usize];
                    let judge_request = ExecRequest {
                        test,
                        test_id: tid.into(),
                    };

                    let test_exec = TestExecutor {
                        exec: judge_request,
                        req,
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
                        .with_context(|| {
                            format!("failed to notify valuer that test {} is done", tid)
                        })?;
                }
                ValuerResponse::Finish => {
                    break;
                }
                ValuerResponse::LiveScore { score } => {
                    self.sender.send(Response::LiveScore(score)).ok();
                }
                ValuerResponse::JudgeLog(judge_log) => judge_logs.push(
                    self.process_judge_log(&judge_log, req)
                        .context("failed to convert valuer judge log to invoker judge log")?,
                ),
            }
        }

        Ok(JudgeLogs(judge_logs))
    }
}

impl InvokeOutcome {
    pub(crate) fn status(&self) -> Option<&Status> {
        match self {
            InvokeOutcome::CompileError(st) => Some(st),
            InvokeOutcome::Judge(judge_logs) => judge_logs.full_status(),
            InvokeOutcome::Fault => None,
        }
    }

    pub(crate) fn score(&self) -> Option<u32> {
        match self {
            InvokeOutcome::CompileError(_) => None,
            InvokeOutcome::Judge(judge_logs) => {
                // TODO: this is wrong.
                Some(judge_logs.full_log().map(|full| full.score).unwrap_or(444))
            }
            InvokeOutcome::Fault => None,
        }
    }

    pub(crate) fn header(&self) -> invoker_api::InvokeOutcomeHeader {
        invoker_api::InvokeOutcomeHeader {
            status: self.status().cloned(),
            score: self.score(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub(crate) enum InvokeOutcome {
    CompileError(Status),
    Judge(JudgeLogs),
    Fault,
}
