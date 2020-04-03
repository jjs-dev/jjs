use super::background_source::BackgroundSource;
use anyhow::Context;
use invoker_api::{CliInvokeTask, InvokeTask};
use log::debug;
use std::sync::Arc;
fn convert_task(cli_invoke_task: CliInvokeTask) -> InvokeTask {
    InvokeTask {
        revision: cli_invoke_task.revision,
        toolchain_id: cli_invoke_task.toolchain_id,
        problem_id: cli_invoke_task.problem_id,
        invocation_id: cli_invoke_task.invocation_id,
        run_dir: cli_invoke_task.run_dir,
        invocation_dir: cli_invoke_task.invocation_dir,
    }
}
async fn read_worker_iteration(state: &BackgroundSource) -> anyhow::Result<()> {
    let mut line = String::new();
    let ret = std::io::stdin()
        .read_line(&mut line)
        .context("failed to read line")?;
    if ret == 0 {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    let task = serde_json::from_str(&line).context("unparseable CliInvokeTask")?;
    debug!("got {:?}", &task);
    let task = convert_task(task);
    state.add_task(task).await;
    Ok(())
}

async fn read_worker_loop(state: Arc<BackgroundSource>) {
    loop {
        if let Err(err) = read_worker_iteration(&*state).await {
            eprintln!("read iteration failed: {:#}", err);
        }
    }
}

async fn print_worker_iteration(state: &BackgroundSource) -> anyhow::Result<()> {
    let msg = match state.pop_msg().await {
        Some(m) => m,
        None => {
            tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
            return Ok(());
        }
    };
    let msg = serde_json::to_string(&msg).context("serialization error")?;
    println!("{}", msg);
    Ok(())
}

async fn print_worker_loop(state: Arc<BackgroundSource>) {
    loop {
        if let Err(err) = print_worker_iteration(&*state).await {
            eprintln!("print iteration failed: {:#}", err);
        }
    }
}

pub struct CliSource(Arc<BackgroundSource>);

impl CliSource {
    pub fn new() -> CliSource {
        let state = Arc::new(BackgroundSource::new());
        let st1 = state.clone();
        let st2 = state.clone();
        tokio::task::spawn(async move {
            read_worker_loop(st1).await;
        });
        tokio::task::spawn(async move {
            print_worker_loop(st2).await;
        });
        CliSource(state)
    }
}

impl Default for CliSource {
    fn default() -> Self {
        CliSource::new()
    }
}

impl std::ops::Deref for CliSource {
    type Target = BackgroundSource;

    fn deref(&self) -> &BackgroundSource {
        &self.0
    }
}
