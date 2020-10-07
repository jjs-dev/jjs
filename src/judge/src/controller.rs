//! Invoker controller
//!
//! Controller is the heart of invoker - it receives InvokeTasks from
//! TaskSources, wraps them into Jobs, schedules this Jobs into workers
//! and publishes Job outcomes.
mod notify;
mod task_loading;
mod toolchains;

use crate::{
    invoker_set::InvokerSet,
    request_handler::{Event, JudgeContext, JudgeOutcome},
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
    pub request: judging_apis::JudgeRequest,
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
        header: judging_apis::JudgeOutcomeHeader,
    ) -> anyhow::Result<()>;

    async fn deliver_live_status_update(
        &self,
        invocation_id: Uuid,
        lsu: judging_apis::LiveStatusUpdate,
    ) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct Controller {
    invoker_set: InvokerSet,
    problem_loader: Arc<problem_loader::Loader>,
    toolchains_dir: Arc<Path>,
    _config: Arc<crate::config::JudgeConfig>,
    // used as RAII resource owner
    _temp_dir: Arc<tempfile::TempDir>,
    toolchain_loader: Arc<toolchains::ToolchainLoader>,
}

fn get_num_cpus() -> usize {
    static NUM_CPUS: once_cell::sync::Lazy<usize> = once_cell::sync::Lazy::new(|| {
        let cnt = num_cpus::get();
        assert_ne!(cnt, 0);
        cnt
    });
    *NUM_CPUS
}

impl Controller {
    pub async fn new(
        cfg_data: util::cfg::CfgData,
        config: Arc<crate::config::JudgeConfig>,
    ) -> anyhow::Result<Controller> {
        let worker_count = match config.managed_invokers {
            Some(cnt) => cnt,
            None => get_num_cpus(),
        };
        info!("Using {} workers", worker_count);

        let invoker_set = {
            let mut builder = InvokerSet::builder(&config);
            for _ in 0..worker_count {
                builder
                    .add_managed_worker()
                    .await
                    .context("failed to start a worker")?;
            }
            builder.build()
        };

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
            invoker_set,
            problem_loader: Arc::new(problem_loader),
            toolchains_dir: cfg_data.data_dir.join("opt").into(),
            _config: config,
            _temp_dir: Arc::new(temp_dir),
            toolchain_loader,
        })
    }

    #[instrument(skip(self, chan))]
    pub async fn exec(self, chan: async_channel::Receiver<JudgeRequestAndCallbacks>) {
        while let Ok(req) = chan.recv().await {
            let this = self.clone();
            let request_id = req.request.request_id;
            tokio::task::spawn(async move {
                if let Err(err) = this.process_request(req).await {
                    tracing::warn!(request_id = %request_id,
                        err = %format_args!("{:#}", err), 
                        "Failed to process a judge request");
                }
            });
        }
    }

    /// This function drives lifecycle of single judge request.
    #[instrument(skip(self, req), fields(request_id=%req.request.request_id))]
    async fn process_request(&self, req: JudgeRequestAndCallbacks) -> anyhow::Result<()> {
        let (low_req, mut exts) = self
            .lower_judge_request(&req)
            .await
            .context("request preprocessing failed")?;

        debug!(lowered_judge_request = ?low_req, "created a lowered judge request");

        let (judge_events_tx, judge_events_rx) = async_channel::bounded(1);
        let engine = self.invoker_set.clone();
        let judge_cx = crate::request_handler::JudgeContext {
            events_tx: judge_events_tx,
            invoker: rpc::Client::new(
                rpc::box_engine(engine),
                "http://does-not-matter".to_string(),
            ),
        };

        // TODO: can we split into LoweredJudgeRequest and Extensions?
        crate::request_handler::do_judge(judge_cx, low_req);
        loop {
            let message = judge_events_rx
                .next()
                .await
                .context("failed to receive next worker message")?;
            match message {
                Event::JudgeDone(judge_outcome) => {
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
                Event::LiveScore(score) => {
                    exts.notifier.set_score(score).await;
                }
                Event::LiveTest(test) => {
                    exts.notifier.set_test(test).await;
                }
                Event::OutcomeHeader(header) => {
                    req.callbacks
                        .add_outcome_header(req.request.request_id, header)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
