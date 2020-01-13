use crate::controller::ControllerDriver;
use anyhow::Context;
use uuid::Uuid;

pub(crate) struct DbDriver {
    db: Box<dyn db::DbConn>,
}

impl DbDriver {
    pub(crate) fn new(db: Box<dyn db::DbConn>) -> DbDriver {
        DbDriver { db }
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
                let invoke_task = invoker_api::InvokeTask {
                    revision: db_invoke_task.revision,
                    run_id: db_invoke_task.run_id,
                    status_update_callback: db_invoke_task.status_update_callback,
                    toolchain_id: db_run.toolchain_id,
                    problem_id: db_run.problem_id,
                    invocation_id,
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

    fn set_run_outcome(
        &self,
        invoke_outcome: crate::worker::InvokeOutcome,
        invocation_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        let mut patch = db::schema::InvocationPatch::default();
        let header = invoke_outcome.header();
        patch
            .state(db::schema::InvocationState::Done)
            .outcome(header)
            .context("failed to set outcome")?;
        self.db
            .inv_update(invocation_id.as_fields().0 as i32, patch)
            .context("failed to store outcome")?;
        Ok(())
    }
}
