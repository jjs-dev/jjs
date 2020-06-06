use crate::controller::{InvocationFinishReason, TaskSource};
use anyhow::Context;
use client::prelude::Sendable;
use uuid::Uuid;

/// Fetches tasks from JJS API
pub struct ApiSource {
    api: client::ApiClient,
    run_mapping: tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, String>>,
}

impl ApiSource {
    pub fn new(api: client::ApiClient) -> ApiSource {
        ApiSource {
            api,
            run_mapping: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl TaskSource for ApiSource {
    async fn load_tasks(&self, cnt: usize) -> anyhow::Result<Vec<invoker_api::InvokeTask>> {
        let runs = client::models::run::Run::pop_run_from_queue()
            .limit(cnt as i64)
            .send(&self.api)
            .await?;

        let mut mapping = self.run_mapping.lock().await;
        let mut tasks = Vec::new();

        for run in runs.object {
            let invocation_id = uuid::Uuid::new_v4();
            mapping.insert(invocation_id, run.id.clone());

            let run_source = client::models::miscellaneous::Miscellaneous::get_run_source()
                .run_id(run.id.clone())
                .send(&self.api)
                .await?
                .object;
            let run_source = base64::decode(&run_source).context("api returned invalid base64")?;

            let task = invoker_api::InvokeTask {
                problem_id: run.problem_name,
                invocation_id,
                revision: 0,
                toolchain_id: run.toolchain_name,
                run_source,
            };

            tasks.push(task);
        }
        Ok(tasks)
    }

    async fn set_finished(
        &self,
        invocation_id: uuid::Uuid,
        _reason: InvocationFinishReason,
    ) -> anyhow::Result<()> {
        let run_id = self
            .run_mapping
            .lock()
            .await
            .remove(&invocation_id)
            .context("corruped run_id_mapping")?;
        let patch = client::models::run_patch::RunPatch::patch_run().run_id(run_id);
        /* TODO
        let state = match reason {
            InvocationFinishReason::CompileError => db::schema::InvocationState::CompileError,
            InvocationFinishReason::Fault => db::schema::InvocationState::InvokeFailed,
            InvocationFinishReason::JudgeDone => db::schema::InvocationState::JudgeDone,
        };
        patch.state(state);
        */
        patch
            .send(&self.api)
            .await
            .context("failed to store outcome")?;
        Ok(())
    }

    async fn add_outcome_header(
        &self,
        _invocation_id: uuid::Uuid,
        _header: invoker_api::InvokeOutcomeHeader,
    ) -> anyhow::Result<()> {
        eprintln!("TODO");
        Ok(())
    }

    async fn deliver_live_status_update(
        &self,
        invocation_id: Uuid,
        _lsu: invoker_api::LiveStatusUpdate,
    ) -> anyhow::Result<()> {
        let mapping = self.run_mapping.lock().await;
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
