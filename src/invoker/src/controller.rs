//! Invoker controller
//!
//! Controller is the heart of invoker - it receives InvokeTasks from
//! TaskSources, wraps them into Jobs, schedules this Jobs into workers
//! and publishes Job outcomes.
mod notify;
mod task_loading;
mod toolchains;

use crate::{
    scheduler::Scheduler,
    worker::{JudgeOutcome, Request, Response},
};
use anyhow::Context;
use notify::Notifier;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{debug, info, instrument};
use uuid::Uuid;

/// Contains additional stuff for controller itself
#[derive(Debug)]
struct LoweredJudgeRequestExtensions {
    notifier: Notifier,
    invocation_dir: PathBuf,
}

pub enum InvocationFinishReason {
    Fault,
    CompileError,
    TestingDone,
}

/// Contains both judging task and back address.
/// Each task source is represented as mpsc channel of `TaskInfo`s
pub struct JudgeRequestAndCallbacks {
    pub request: invoker_api::JudgeRequest,
    pub callbacks: Arc<dyn JudgeResponseCallbacks>,
}

impl std::fmt::Debug for JudgeRequestAndCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JudgeRequestAndCallbacks")
            .field("request", &self.request)
            .field("handler", &"..")
            .finish()
    }
}

#[async_trait::async_trait]
pub trait JudgeResponseCallbacks: Send + Sync {
    async fn set_finished(
        &self,
        invocation_id: Uuid,
        reason: InvocationFinishReason,
    ) -> anyhow::Result<()>;

    /// Called when a judge log is available.
    /// kinds can be duplicated after a judge fault.
    async fn add_outcome_header(
        &self,
        invocation_id: Uuid,
        header: invoker_api::JudgeOutcomeHeader,
    ) -> anyhow::Result<()>;

    async fn deliver_live_status_update(
        &self,
        invocation_id: Uuid,
        lsu: invoker_api::LiveStatusUpdate,
    ) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct Controller {
    scheduler: Arc<Scheduler>,
    problem_loader: Arc<problem_loader::Loader>,
    toolchains_dir: Arc<Path>,
    _config: Arc<crate::config::InvokerConfig>,
    // used as RAII resource owner
    _temp_dir: Arc<tempfile::TempDir>,
    toolchain_loader: Arc<toolchains::ToolchainLoader>,
}

fn get_num_cpus() -> usize {
    static CACHE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let old = CACHE.load(std::sync::atomic::Ordering::Relaxed);
    if old != 0 {
        return old;
    }
    let corr = num_cpus::get();
    assert_ne!(corr, 0);
    CACHE.store(corr, std::sync::atomic::Ordering::Relaxed);
    corr
}

impl Controller {
    pub async fn new(
        cfg_data: util::cfg::CfgData,
        config: Arc<crate::config::InvokerConfig>,
    ) -> anyhow::Result<Controller> {
        let worker_count = match config.workers {
            Some(cnt) => cnt,
            None => get_num_cpus(),
        };
        info!("Using {} workers", worker_count);
        let mut scheduler = Scheduler::new(&config).context("failed to initialize Scheduler")?;
        for _ in 0..worker_count {
            scheduler
                .add_worker()
                .await
                .context("failed to start a worker")?;
        }
        let scheduler = Arc::new(scheduler);

        let temp_dir = tempfile::TempDir::new().context("can not find temporary dir")?;

        let problem_loader =
            problem_loader::Loader::from_config(&config.problems, temp_dir.path().join("problems"))
                .await
                .context("can not create ProblemLoader")?;
        let toolchain_loader = Arc::new(
            toolchains::ToolchainLoader::new()
                .await
                .context("toolchain loader initialization error")?,
        );
        Ok(Controller {
            scheduler,
            problem_loader: Arc::new(problem_loader),
            toolchains_dir: cfg_data.data_dir.join("opt").into(),
            _config: config,
            _temp_dir: Arc::new(temp_dir),
            toolchain_loader,
        })
    }

    #[instrument(skip(self, chan))]
    pub fn exec_on(self, chan: async_mpmc::Receiver<JudgeRequestAndCallbacks>) {
        chan.process_all(move |req| {
            let this = self.clone();

            async move {
                let request_id = req.request.request_id;
                if let Err(err) = this.process_request(req).await {
                    tracing::warn!(request_id = %request_id,
                    err = %format_args!("{:#}", err), 
                    "Failed to process a judge request");
                }
            }
        });
    }

    /// This function drives lifecycle of single judge request.
    #[instrument(skip(self, req), fields(request_id=%req.request.request_id))]
    async fn process_request(&self, req: JudgeRequestAndCallbacks) -> anyhow::Result<()> {
        let (low_req, mut exts) = self
            .lower_judge_request(&req)
            .await
            .context("request preprocessing failed")?;

        debug!(lowered_judge_request = ?low_req, "created a lowered judge request");

        // TODO currently the process of finding a worker is unfair
        // we should fix it e.g. using a semaphore which permits finding
        // worker.
        let worker = self.scheduler.find_free_worker().await;
        // TODO: can we split into LoweredJudgeRequest and Extensions?
        let mut responses = worker
            .send(Request::Judge(low_req))
            .await
            .context("failed to submit lowered judge request")?;
        loop {
            let message = responses
                .next()
                .await
                .context("failed to receive next worker message")?;
            match message {
                Response::JudgeDone(judge_outcome) => {
                    debug!("Publising: JudgeOutcome {:?}", &judge_outcome);
                    let reason = match judge_outcome {
                        JudgeOutcome::Fault => InvocationFinishReason::Fault,
                        JudgeOutcome::TestingDone => InvocationFinishReason::TestingDone,
                        JudgeOutcome::CompileError(_) => InvocationFinishReason::CompileError,
                    };
                    req.callbacks
                        .set_finished(req.request.request_id, reason)
                        .await
                        .context("failed to set run outcome in DB")?;
                    break;
                }
                Response::LiveScore(score) => {
                    exts.notifier.set_score(score).await;
                }
                Response::LiveTest(test) => {
                    exts.notifier.set_test(test).await;
                }
                Response::OutcomeHeader(header) => {
                    req.callbacks
                        .add_outcome_header(req.request.request_id, header)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
