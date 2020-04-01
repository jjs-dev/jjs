mod notify;
mod tasks;

use crate::worker::{InvokeOutcome, InvokeRequest, Request, Response};
use anyhow::Context;
use log::{debug, error, info, warn};
use notify::Notifier;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use uuid::Uuid;

enum WorkerState {
    /// Worker is ready for new tasks
    Idle,
    /// Worker is invoking run
    Invoke(Box<ExtendedInvokeRequest>),
    /// Worker has crashed
    Crash,
}

impl WorkerState {
    fn is_idle(&self) -> bool {
        match self {
            WorkerState::Idle => true,
            _ => false,
        }
    }

    fn is_working(&self) -> bool {
        match self {
            WorkerState::Invoke(_) => true,
            _ => false,
        }
    }
}

struct WorkerInfo {
    state: WorkerState,
    child_stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
    child_stdin: tokio::process::ChildStdin,
}

impl WorkerInfo {
    async fn recv(&mut self) -> anyhow::Result<Option<Response>> {
        let mut line = String::new();
        let read_line_fut = self.child_stdout.read_line(&mut line);

        match tokio::time::timeout(std::time::Duration::from_millis(10), read_line_fut).await {
            Ok(read) => {
                read?;
                Ok(serde_json::from_str(&line).context("parse error")?)
            }
            Err(_elapsed) => Ok(None),
        }
    }

    async fn send(&mut self, req: Request) -> anyhow::Result<()> {
        let mut data = serde_json::to_vec(&req)?;
        data.push(b'\n');
        self.child_stdin.write_all(&data).await?;
        Ok(())
    }
}

/// Contains `RunInfo` for worker + stuff for controller itself
#[derive(Debug)]
struct ExtendedInvokeRequest {
    inner: InvokeRequest,
    revision: u32,
    notifier: Notifier,
    invocation_dir: PathBuf,
    task_source_id: usize,
}

struct ControllerQueues {
    invoke_queue: Mutex<VecDeque<ExtendedInvokeRequest>>,
    publish_queue: Mutex<VecDeque<(InvokeOutcome, ExtendedInvokeRequest)>>,
}

impl ControllerQueues {
    fn new() -> ControllerQueues {
        ControllerQueues {
            invoke_queue: Mutex::new(VecDeque::new()),
            publish_queue: Mutex::new(VecDeque::new()),
        }
    }

    fn invoke(&self) -> std::sync::MutexGuard<VecDeque<ExtendedInvokeRequest>> {
        self.invoke_queue.lock().unwrap()
    }

    fn publish(&self) -> std::sync::MutexGuard<VecDeque<(InvokeOutcome, ExtendedInvokeRequest)>> {
        self.publish_queue.lock().unwrap()
    }
}

pub enum InvocationFinishReason {
    Fault,
    CompileError,
    JudgeDone,
}

#[async_trait::async_trait]
pub trait TaskSource {
    async fn load_tasks(&self, cnt: usize) -> anyhow::Result<Vec<invoker_api::InvokeTask>>;

    async fn set_finished(
        &self,
        invocation_id: Uuid,
        reason: InvocationFinishReason,
    ) -> anyhow::Result<()>;

    async fn add_outcome_header(
        &self,
        invocation_id: Uuid,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<S: TaskSource + Send + Sync + 'static, T: std::ops::Deref<Target = S> + Send + Sync> TaskSource
    for T
{
    async fn load_tasks(&self, cnt: usize) -> anyhow::Result<Vec<invoker_api::InvokeTask>> {
        (**self).load_tasks(cnt).await
    }

    async fn set_finished(
        &self,
        invocation_id: Uuid,
        reason: InvocationFinishReason,
    ) -> anyhow::Result<()> {
        let inner: &S = self.deref();
        inner.set_finished(invocation_id, reason).await
    }

    async fn add_outcome_header(
        &self,
        invocation_id: Uuid,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> anyhow::Result<()> {
        let inner: &S = self.deref();
        inner.add_outcome_header(invocation_id, header).await
    }
}

pub struct Controller {
    workers: Vec<WorkerInfo>,
    sources: Vec<Box<dyn TaskSource>>,
    entity_loader: Arc<entity::Loader>,
    stop_flag: Arc<AtomicBool>,
    queues: Arc<ControllerQueues>,
    problem_loader: Arc<problem_loader::Loader>,
    global_files_dir: Arc<Path>,
    toolchains_dir: Arc<Path>,
}

impl Controller {
    pub fn new(
        sources: Vec<Box<dyn TaskSource>>,
        cfg_data: util::cfg::CfgData,
        worker_count: usize,
    ) -> anyhow::Result<Controller> {
        let mut workers = Vec::new();
        for _ in 0..worker_count {
            let mut child = tokio::process::Command::new(std::env::current_exe()?)
                .env("__JJS_WORKER", "1")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .context("failed to spawn worker")?;
            let info = WorkerInfo {
                state: WorkerState::Idle,
                child_stdin: child.stdin.take().expect("child stdin was captured"),
                child_stdout: tokio::io::BufReader::new(
                    child.stdout.take().expect("child stdout was captured"),
                ),
            };
            workers.push(info);
        }

        Ok(Controller {
            sources,
            workers,
            entity_loader: Arc::new(cfg_data.entity_loader),
            problem_loader: Arc::new(cfg_data.problem_loader),
            stop_flag: Arc::new(AtomicBool::new(false)),
            queues: Arc::new(ControllerQueues::new()),
            global_files_dir: cfg_data.install_dir.into(),
            toolchains_dir: cfg_data.data_dir.join("opt").into(),
        })
    }

    pub async fn run_forever(mut self) {
        if let Err(err) = self.setup_signal() {
            error!("SIGTERM handler is not registered: {}", err);
        }
        loop {
            if !self.should_run() {
                break;
            }
            let sleep = match self.tick().await {
                Err(e) => {
                    warn!("Tick failed: {:#}", e);
                    true
                }
                Ok(flag) => !flag,
            };
            if sleep {
                // TODO adaptive sleep duration
                let sleep_time = std::time::Duration::from_secs(2);
                std::thread::sleep(sleep_time);
            }
        }
    }

    // this function calls several `tick` functions
    async fn tick(&mut self) -> anyhow::Result<bool> {
        // did we have any updates?
        let mut flag = false;
        flag = flag || self.tick_poll_workers().await?;
        flag = flag || self.tick_publish_outcome().await?;
        flag = flag || self.tick_send_invoke_request().await?;
        flag = flag || self.tick_get_tasks().await?;
        Ok(flag)
    }

    fn should_run(&self) -> bool {
        !self.stop_flag.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn setup_signal(&mut self) -> anyhow::Result<()> {
        let sigterm_sig_id = nix::sys::signal::SIGTERM as i32;
        signal_hook::flag::register(sigterm_sig_id, Arc::clone(&self.stop_flag))
            .context("Failed to registrer SIGTERM handler")?;
        Ok(())
    }

    fn find_free_worker(&self) -> Option<usize> {
        self.workers
            .iter()
            .position(|worker| worker.state.is_idle())
    }

    async fn load_tasks(
        &mut self,
        mut limit: usize,
    ) -> anyhow::Result<Vec<(invoker_api::InvokeTask, usize)>> {
        let mut tasks = Vec::new();
        for (source_id, source) in self.sources.iter().enumerate() {
            let chunk = source.load_tasks(limit).await?;
            limit -= chunk.len();
            tasks.extend(chunk.into_iter().map(|task| (task, source_id)));
        }
        Ok(tasks)
    }

    async fn tick_send_invoke_request(&mut self) -> anyhow::Result<bool> {
        let free_worker = match self.find_free_worker() {
            Some(fw) => fw,
            None => return Ok(false),
        };
        let invoke_request = match self.queues.invoke().pop_front() {
            Some(inv_out) => inv_out,
            None => return Ok(false),
        };

        let worker = &mut self.workers[free_worker];
        let req = Request::Invoke(invoke_request.inner.clone());
        worker
            .send(req)
            .await
            .context("failed to send request to invoker")?;
        worker.state = WorkerState::Invoke(Box::new(invoke_request));
        Ok(true)
    }

    fn invoke_queue_size(&self) -> usize {
        self.workers.len()
    }

    async fn tick_get_tasks(&mut self) -> anyhow::Result<bool> {
        if self.queues.invoke().len() >= self.invoke_queue_size() {
            return Ok(false);
        }
        let cnt = self.invoke_queue_size() - self.queues.invoke().len();
        debug!("Searching for tasks (limit {})", cnt);
        let new_tasks = self.load_tasks(cnt).await?;
        if new_tasks.is_empty() {
            debug!("No new tasks");
            return Ok(false);
        } else {
            info!("{} new tasks discovered", new_tasks.len());
        }
        for (invoke_task, task_source_id) in new_tasks {
            let extended_invoke_request = self.fetch_run_info(&invoke_task, task_source_id)?;
            self.queues.invoke().push_back(extended_invoke_request);
        }
        Ok(true)
    }

    async fn process_worker_message(
        &mut self,
        msg: Response,
        worker_id: usize,
    ) -> anyhow::Result<()> {
        debug!("Processing message {:?} from worker {}", &msg, worker_id);
        let worker = &mut self.workers[worker_id];
        let old_state = std::mem::replace(&mut worker.state, WorkerState::Idle);
        let mut req = match old_state {
            WorkerState::Invoke(req) => req,
            WorkerState::Idle => panic!("WorkerState is Idle, but msg {:?} was received", msg),
            WorkerState::Crash => panic!("WorkerState is Crash, but msg {:?} was received", msg),
        };
        match msg {
            Response::Invoke(outcome) => {
                self.copy_invocation_data_dir_to_shared_fs(
                    &req.inner.out_dir,
                    &req.invocation_dir,
                )?;
                self.queues.publish().push_back((outcome, *req));
            }
            Response::LiveScore(score) => {
                req.notifier.set_score(score).await;
                worker.state = WorkerState::Invoke(req);
            }
            Response::LiveTest(test) => {
                req.notifier.set_test(test).await;
                worker.state = WorkerState::Invoke(req);
            }
            Response::OutcomeHeader(header) => {
                self.sources[req.task_source_id]
                    .add_outcome_header(req.inner.invocation_id, header)
                    .await?;
                let dir = req.inner.out_dir.clone();
                let invocation_dir = req.invocation_dir.clone();
                worker.state = WorkerState::Invoke(req);
                self.copy_invocation_data_dir_to_shared_fs(&dir, &invocation_dir)?;
            }
        }
        debug!("Processing done");
        Ok(())
    }

    async fn tick_poll_workers(&mut self) -> anyhow::Result<bool> {
        let mut msgs = Vec::new();
        const MAX_MSGS_BATCH: usize = 5;
        for (i, worker) in self.workers.iter_mut().enumerate() {
            if !worker.state.is_working() {
                continue;
            }
            loop {
                if msgs.len() >= MAX_MSGS_BATCH {
                    break;
                }
                match worker.recv().await {
                    Ok(Some(msg)) => {
                        msgs.push((i, msg));
                    }
                    Ok(None) => break,
                    Err(err) => {
                        error!("worker {} crashed: failed to receive Response: {}", i, err);
                        worker.state = WorkerState::Crash;
                        break;
                    }
                }
            }
        }
        let emp = msgs.is_empty();
        for msg in msgs {
            self.process_worker_message(msg.1, msg.0).await?;
        }
        Ok(!emp)
    }

    async fn tick_publish_outcome(&mut self) -> anyhow::Result<bool> {
        let (invoke_outcome, ext_inv_req) = match self.queues.publish().pop_front() {
            Some(r) => r,
            None => return Ok(false),
        };
        debug!(
            "Publising: InvokeOutcome {:?} {:?}",
            &invoke_outcome, &ext_inv_req
        );
        let reason = match invoke_outcome {
            InvokeOutcome::Fault => InvocationFinishReason::Fault,
            InvokeOutcome::Judge => InvocationFinishReason::JudgeDone,
            InvokeOutcome::CompileError(_) => InvocationFinishReason::CompileError,
        };
        self.sources[ext_inv_req.task_source_id]
            .set_finished(ext_inv_req.inner.invocation_id, reason)
            .await
            .context("failed to set run outcome in DB")?;

        Ok(true)
    }

    fn copy_invocation_data_dir_to_shared_fs(
        &self,
        temp_dir: &Path,
        invocation_dir: &Path,
    ) -> anyhow::Result<()> {
        if let Err(err) = std::fs::create_dir(invocation_dir) {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                anyhow::bail!(
                    "failed to create target dir {}: {}",
                    invocation_dir.display(),
                    err
                );
            }
        }
        let from: Vec<_> = std::fs::read_dir(temp_dir)
            .context("failed to list source directory")?
            .map(|x| x.map(|y| y.path()))
            .collect::<Result<_, _>>()?;
        debug!(
            "Copying from {} to {}",
            temp_dir.display(),
            invocation_dir.display()
        );
        let mut opts = fs_extra::dir::CopyOptions::new();
        opts.overwrite = true;
        fs_extra::copy_items(&from, invocation_dir, &opts)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            .context("failed to copy")?;
        Ok(())
    }
}
