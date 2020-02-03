use crate::controller::{ControllerDriver, InvocationFinishReason};
use anyhow::Context;
use std::sync::Arc;
use uuid::Uuid;

pub struct DbDriver {
    db: Box<dyn db::DbConn>,
    config: Arc<cfg::Config>,
}

impl DbDriver {
    pub fn new(db: Box<dyn db::DbConn>, config: Arc<cfg::Config>) -> DbDriver {
        DbDriver { db, config }
    }
}

impl ControllerDriver for DbDriver {
    fn load_tasks(&self, mut cnt: usize) -> anyhow::Result<Vec<invoker_api::InvokeTask>> {
        let mut new_tasks = Vec::new();
        const WINDOW_SIZE: u32 = 10;
        const WINDOW_STEP: u32 = 9;
        // https://github.com/rust-lang/rust-clippy/issues/5064
        // assert!(WINDOW_STEP < WINDOW_SIZE);
        let mut offset = 0;
        while cnt > 0 {
            let mut visited_count = 0;
            let chunk: Vec<db::schema::Invocation> =
                self.db
                    .inv_find_waiting(offset, WINDOW_SIZE, &mut |_invocation| {
                        visited_count += 1;
                        if cnt > 0 {
                            return Ok(true);
                        }
                        cnt -= 1;
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
            if visited_count == 0 {
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
