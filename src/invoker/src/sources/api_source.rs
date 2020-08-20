use crate::controller::{InvocationFinishReason, JudgeRequestAndCallbacks, JudgeResponseCallbacks};
use anyhow::Context;
use client::prelude::Sendable;
use std::sync::Arc;
use tracing::{info, instrument, warn};
use uuid::Uuid;

struct Inner {
    api: client::ApiClient,
    run_mapping: tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, String>>,
}

/// Fetches tasks from JJS API
pub struct ApiSource {
    inner: Arc<Inner>,
    chan: async_mpmc::Sender<JudgeRequestAndCallbacks>,
}

// double Arc, but who cares?
struct Callbacks {
    inner: Arc<Inner>,
}

#[async_trait::async_trait]
impl JudgeResponseCallbacks for Callbacks {
    async fn set_finished(
        &self,
        invocation_id: uuid::Uuid,
        _reason: InvocationFinishReason,
    ) -> anyhow::Result<()> {
        let run_id = self
            .inner
            .run_mapping
            .lock()
            .await
            .remove(&invocation_id)
            .context("corrupted run_id_mapping")?;
        let patch = client::models::RunPatch::patch_run().run_id(run_id);
        /* TODO
        let state = match reason {
            InvocationFinishReason::CompileError => db::schema::InvocationState::CompileError,
            InvocationFinishReason::Fault => db::schema::InvocationState::InvokeFailed,
            InvocationFinishReason::JudgeDone => db::schema::InvocationState::JudgeDone,
        };
        patch.state(state);
        */
        patch
            .send(&self.inner.api)
            .await
            .context("failed to store outcome")?;
        Ok(())
    }

    async fn add_outcome_header(
        &self,
        invocation_id: uuid::Uuid,
        header: invoker_api::JudgeOutcomeHeader,
    ) -> anyhow::Result<()> {
        let run_id = self
            .inner
            .run_mapping
            .lock()
            .await
            .get(&invocation_id)
            .context("corrupted run_id_mapping")?
            .clone();
        client::models::RunPatch::patch_run()
            .run_id(run_id)
            .status(
                vec![
                    vec![
                        header.kind.as_str(),
                        &format!("{}:{}", header.status.kind, header.status.code),
                    ]
                    .into_iter(),
                ]
                .into_iter(),
            )
            .send(&self.inner.api)
            .await
            .context("failed to send outcome to API")?;
        Ok(())
    }

    async fn deliver_live_status_update(
        &self,
        invocation_id: Uuid,
        _lsu: invoker_api::LiveStatusUpdate,
    ) -> anyhow::Result<()> {
        let mapping = self.inner.run_mapping.lock().await;
        let run_id = match mapping.get(&invocation_id) {
            Some(id) => id,
            None => {
                anyhow::bail!("warning: invocation_id {} not found", invocation_id);
            }
        };
        let _key = format!("lsu-{}", run_id);
        eprintln!("TODO");
        Ok(())
    }
}

impl ApiSource {
    pub fn new(
        api: client::ApiClient,
        chan: async_mpmc::Sender<JudgeRequestAndCallbacks>,
    ) -> ApiSource {
        let inner = Inner {
            api,

            run_mapping: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        };
        let inner = Arc::new(inner);
        ApiSource { chan, inner }
    }

    fn make_callbacks(&self) -> Arc<dyn JudgeResponseCallbacks> {
        Arc::new(Callbacks {
            inner: self.inner.clone(),
        })
    }

    async fn get_tasks_from_api(&self) -> anyhow::Result<Vec<invoker_api::JudgeRequest>> {
        let runs = client::models::Run::pop_run_from_queue()
            .limit(1_i64)
            .send(&self.inner.api)
            .await
            .context("failed to get task list")?;

        let mut mapping = self.inner.run_mapping.lock().await;
        let mut tasks = Vec::new();

        for run in runs.object {
            let request_id = uuid::Uuid::new_v4();
            mapping.insert(request_id, run.id.clone());

            let run_source = client::models::Misc::get_run_source()
                .run_id(run.id.clone())
                .send(&self.inner.api)
                .await
                .context("run source not available")?
                .object;
            let run_source = base64::decode(&run_source).context("api returned invalid base64")?;
            let toolchain = client::models::Toolchain::get_toolchain()
                .toolchain_id(&run.toolchain_name)
                .send(&self.inner.api)
                .await
                .context("toolchain resolution failed")?;

            let task = invoker_api::JudgeRequest {
                problem_id: run.problem_name,
                request_id,
                revision: 0,
                toolchain_id: toolchain.image.clone(),
                run_source,
            };

            tasks.push(task);
        }
        std::mem::drop(mapping);
        if tasks.is_empty() {
            // hack, but will be rewritten anyway
            tokio::time::delay_for(std::time::Duration::from_secs(3)).await;
        }
        Ok(tasks)
    }

    async fn tick(&mut self) -> anyhow::Result<()> {
        let reqs = self
            .get_tasks_from_api()
            .await
            .context("failed to fetch tasks")?;

        for request in reqs {
            let req_cbs = JudgeRequestAndCallbacks {
                request,
                callbacks: self.make_callbacks(),
            };
            self.chan.send(req_cbs);
        }
        Ok(())
    }

    /// Fetches tasks and posts results back in loop, until cancelled.
    #[instrument(skip(self))]
    pub async fn run(mut self, cancel: tokio::sync::CancellationToken) {
        info!("Starting loop");
        loop {
            let tick_res = tokio::select! {
                res = self.tick() => res,
                _ = cancel.cancelled() => {
                    info!("Cancelled");
                    return;
                }
            };
            if let Err(err) = tick_res {
                warn!("tick failed: {:#}", err);
            }
        }
    }
}
