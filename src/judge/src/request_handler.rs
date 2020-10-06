//! This module and its children are responsible for creating requests to Worker.
mod compiler;
mod exec_test;
mod transform_judge_log;
mod valuer;

use self::{
    compiler::{BuildOutcome, Compiler},
    exec_test::{exec, ExecRequest},
};
use judging_apis::Status;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};
use tracing::debug;

/// Allows sending requests to invoker
// TODO upstream BoxedEngine to jjs-commons
pub type InvokerClient = rpc::Client<rpc::BoxEngine>;

pub(crate) struct JudgeContext {
    /// Can be used to send requests to invoker
    pub(crate) invoker: InvokerClient,
    /// Channel that should be used for sending updates
    pub(crate) events_tx: async_channel::Sender<Event>,
}

/// Note: this is not `judging_apis::invoke::Command`, it is higher-level.
#[derive(Debug)]
pub(crate) struct Command {
    pub argv: Vec<String>,
    pub env: Vec<String>,
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
