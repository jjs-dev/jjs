use super::background_source::BackgroundSource;
use anyhow::Context;
use invoker_api::{CliInvokeTask, InvokeTask};
use slog_scope::debug;
use std::sync::Arc;
fn convert_task(cli_invoke_task: CliInvokeTask) -> InvokeTask {
    InvokeTask {
        revision: cli_invoke_task.revision,
        status_update_callback: None,
        toolchain_id: cli_invoke_task.toolchain_id,
        problem_id: cli_invoke_task.problem_id,
        invocation_id: cli_invoke_task.invocation_id,
        run_dir: cli_invoke_task.run_dir,
        invocation_dir: cli_invoke_task.invocation_dir,
    }
}
fn read_worker_iteration(state: &BackgroundSource) -> anyhow::Result<()> {
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
    state.add_task(task);
    Ok(())
}

fn read_worker_loop(state: Arc<BackgroundSource>) {
    loop {
        if let Err(err) = read_worker_iteration(&*state) {
            eprintln!("read iteration failed: {:#}", err);
        }
    }
}

fn print_worker_iteration(state: &BackgroundSource) -> anyhow::Result<()> {
    let msg = match state.pop_msg() {
        Some(m) => m,
        None => {
            std::thread::sleep(std::time::Duration::from_secs(1));
            return Ok(());
        }
    };
    let msg = serde_json::to_string(&msg).context("serialization error")?;
    println!("{}", msg);
    Ok(())
}

fn print_worker_loop(state: Arc<BackgroundSource>) {
    loop {
        if let Err(err) = print_worker_iteration(&*state) {
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
        std::thread::spawn(move || {
            read_worker_loop(st1);
        });
        std::thread::spawn(move || {
            print_worker_loop(st2);
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
