use crate::controller::{InvocationFinishReason, TaskSource};
use anyhow::Context;
use std::sync::Arc;
use uuid::Uuid;

pub struct DbSource {
    db: Box<dyn db::DbConn>,
    config: Arc<cfg::Config>,
}

impl DbSource {
    pub fn new(db: Box<dyn db::DbConn>, config: Arc<cfg::Config>) -> DbSource {
        DbSource { db, config }
    }
}

impl TaskSource for DbSource {
    fn load_tasks(&self, mut cnt: usize) -> anyhow::Result<Vec<invoker_api::InvokeTask>> {
        let mut new_tasks = Vec::new();
        const WINDOW_SIZE: u32 = 10;
        const WINDOW_STEP: u32 = 9;
        {
            #[allow(dead_code)]
            const ASSERT_SIZE_IS_GREATER_THAN_STEP: usize =
                (WINDOW_SIZE - WINDOW_STEP - 1) as usize;
        }
        let mut offset = 0;
        while cnt > 0 {
            let mut discovered_new_tasks = false;
            let chunk: Vec<db::schema::Invocation> =
                self.db
                    .inv_find_waiting(offset, WINDOW_SIZE, &mut |_invocation| {
                        if cnt > 0 {
                            discovered_new_tasks = true;
                            cnt -= 1;
                            return Ok(true);
                        }
                        Ok(false)
                    })?;
            for invocation in chunk {
                let db_invoke_task = invocation.invoke_task()?;
                let db_run = self.db.run_load(db_invoke_task.run_id as i32)?;
                let invocation_id = Uuid::from_fields(invocation.id as u32, 0, 0, &[0; 8])
                    .expect("this call is always correct");
                let run_dir = self.config.sysroot.join("var/submissions");
                let run_dir = run_dir.join(&format!("s-{}", db_invoke_task.run_id));
                let invocation_dir = run_dir.join(&format!("i-{}", db_invoke_task.revision));
                let invoke_task = invoker_api::InvokeTask {
                    revision: db_invoke_task.revision,
                    status_update_callback: db_invoke_task.status_update_callback,
                    toolchain_id: db_run.toolchain_id,
                    problem_id: db_run.problem_id,
                    invocation_id,
                    run_dir,
                    invocation_dir,
                };
                new_tasks.push(invoke_task);
            }
            if !discovered_new_tasks {
                break;
            }
            offset += WINDOW_STEP;
        }
        Ok(new_tasks)
    }

    fn set_finished(
        &self,
        invocation_id: uuid::Uuid,
        reason: InvocationFinishReason,
    ) -> anyhow::Result<()> {
        let mut patch = db::schema::InvocationPatch::default();
        let state = match reason {
            InvocationFinishReason::CompileError => db::schema::InvocationState::CompileError,
            InvocationFinishReason::Fault => db::schema::InvocationState::InvokeFailed,
            InvocationFinishReason::JudgeDone => db::schema::InvocationState::JudgeDone,
        };
        patch.state(state);
        self.db
            .inv_update(invocation_id.as_fields().0 as i32, patch)
            .context("failed to store outcome")?;
        Ok(())
    }

    fn add_outcome_header(
        &self,
        invocation_id: uuid::Uuid,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> anyhow::Result<()> {
        self.db
            .inv_add_outcome_header(invocation_id.as_fields().0 as i32, header)
    }
}
