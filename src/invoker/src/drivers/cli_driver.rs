use crate::{controller::ControllerDriver, worker::InvokeOutcome};
use anyhow::Context;
use invoker_api::{CliInvokeTask, InvokeTask};
use serde::Serialize;
use slog_scope::debug;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

struct CliDriverState {
    queue: VecDeque<CliInvokeTask>,
}
pub(crate) struct CliDriver {
    state: Arc<Mutex<CliDriverState>>,
}

fn worker_iteration(state: &Mutex<CliDriverState>) -> anyhow::Result<()> {
    let mut line = String::new();
    let ret = std::io::stdin()
        .read_line(&mut line)
        .context("failed to read line")?;
    if ret == 0 {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    let task = serde_json::from_str(&line).context("unparseable CliInvokeTask")?;
    let mut q = state.lock().unwrap();
    debug!("got {:?}", &task);
    q.queue.push_back(task);
    Ok(())
}

fn worker_loop(state: Arc<Mutex<CliDriverState>>) {
    loop {
        if let Err(err) = worker_iteration(&*state) {
            eprintln!("iteration failed: {:#}", err);
        }
    }
}

impl CliDriver {
    pub fn new() -> anyhow::Result<CliDriver> {
        let state = CliDriverState {
            queue: VecDeque::new(),
        };
        let state = Arc::new(Mutex::new(state));
        let driver = CliDriver {
            state: state.clone(),
        };
        std::thread::spawn(move || {
            worker_loop(state);
        });
        Ok(driver)
    }

    fn convert_task(cli_invoke_task: CliInvokeTask) -> InvokeTask {
        InvokeTask {
            revision: cli_invoke_task.revision,
            run_id: cli_invoke_task.run_id,
            status_update_callback: None,
            toolchain_id: cli_invoke_task.toolchain_id,
            problem_id: cli_invoke_task.problem_id,
            invocation_id: cli_invoke_task.invocation_id,
        }
    }
}

#[derive(Serialize)]
struct Message {
    invocation_id: Uuid,
    run_outcome: InvokeOutcome,
}

impl ControllerDriver for CliDriver {
    fn load_tasks(&self, cnt: usize) -> anyhow::Result<Vec<InvokeTask>> {
        let mut q = self.state.lock().unwrap();
        let q = &mut q.queue;
        Ok(q.drain(0..cnt.min(q.len()))
            .map(Self::convert_task)
            .collect())
    }

    fn set_run_outcome(
        &self,
        run_outcome: InvokeOutcome,
        invocation_id: Uuid,
    ) -> anyhow::Result<()> {
        let msg = Message {
            run_outcome,
            invocation_id,
        };
        let msg = serde_json::to_string(&msg).context("serialization error")?;
        println!("{}", msg);
        Ok(())
    }
}
