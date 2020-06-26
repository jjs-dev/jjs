// maybe it's overengineered but it's fun

use crate::{
    config::InvokerConfig,
    worker::{Request, Response},
};
use anyhow::Context as _;
use std::sync::atomic::{AtomicU8, Ordering::SeqCst};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    sync::Mutex,
};
use tracing::{debug, instrument};
/// Scheduler is responsible for finding a suitable worker for a task
pub struct Scheduler {
    workers: Vec<WorkerInfo>,
    /// We need it because we must pass it to a worker.
    // TODO: ideally we do not want to pass any configs to a worker
    config: String,
    /// these field is used to signal that a worker is reclaimed
    worker_reclamation: (multiwake::Sender, multiwake::Receiver),
}

impl Scheduler {
    /// Creates new Scheduler with empty `workers` set
    pub fn new(config: &InvokerConfig) -> anyhow::Result<Self> {
        let config = serde_json::to_string(&config).context("failed to serialize InvokerConfig")?;
        Ok(Scheduler {
            workers: vec![],
            config,
            worker_reclamation: multiwake::multiwake(),
        })
    }

    /// Starts new worker process and adds it to this scheduler
    #[instrument(skip(self))]
    pub async fn add_worker(&mut self) -> anyhow::Result<()> {
        let mut child = tokio::process::Command::new(std::env::current_exe()?)
            .env("__JJS_WORKER", "1")
            .env("__JJS_WORKER_INVOKER_CONFIG", &self.config)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn worker")?;
        let info = WorkerInfo {
            state: WorkerState::new(WorkerStateKind::Idle),
            child_stdin: Mutex::new(child.stdin.take().expect("child stdin was captured")),
            child_stdout: Mutex::new(tokio::io::BufReader::new(
                child.stdout.take().expect("child stdout was captured"),
            )),
        };
        self.workers.push(info);
        Ok(())
    }

    /// Tries to find a free worker. On success, returns `FreeWorkerHandle`,
    /// which can be used to send requests to that worker.
    #[instrument(skip(self))]
    pub async fn find_free_worker(&self) -> FreeWorkerHandle<'_> {
        let mut receiver = self.worker_reclamation.1.clone();
        let mut attempt_id = 0u32;
        loop {
            debug!(attempt_id, "scanning all workers");
            attempt_id += 1;
            for worker in &self.workers {
                if let Some(handle) = worker.try_lock(self.worker_reclamation.0.clone()) {
                    return handle;
                }
            }
            receiver.wait().await;
        }
    }
}

/// This handle logically owns free worker and can be used to send it requests.
pub struct FreeWorkerHandle<'a> {
    /// reference to the worker
    worker: &'a WorkerInfo,
    /// Used to notify that worker is reclaimed
    notify: multiwake::Sender,
}

impl Drop for FreeWorkerHandle<'_> {
    fn drop(&mut self) {
        // true if worker can be reused later.
        // Theoretically, it should always be the case when handle is dropped.
        // However, due to bugs in JJS or problems in environment task, using
        // this worker can fail in unexpected manner, leaving worker in
        // inconsistent state. This flag implements conservative strategy
        // which allows us to avoid such situations.
        let reclaimable = matches!(
            self.worker.state.load(),
            WorkerStateKind::Idle | WorkerStateKind::Locked
        );
        if reclaimable {
            tracing::debug!("Reclaiming worker");
            self.worker.state.store(WorkerStateKind::Idle);
            self.notify.wake();
        } else {
            tracing::warn!("Leaking worker because it is not in reclaimable state");
            self.worker.state.store(WorkerStateKind::Crash);
        }
    }
}

impl<'a> FreeWorkerHandle<'a> {
    /// Sends request to worker, returning "stream" of responses
    pub(crate) async fn send(self, req: Request) -> anyhow::Result<WorkerResponses<'a>> {
        self.worker.state.store(WorkerStateKind::Judge);
        self.worker
            .send(req)
            .await
            .map_err(|err| {
                tracing::warn!("request not delivered, marking worker as crashed");
                self.worker.state.store(WorkerStateKind::Crash);
                err
            })
            .context("failed to send request")?;
        Ok(WorkerResponses { handle: Some(self) })
    }
}

/// Provides access to worker responses
pub struct WorkerResponses<'a> {
    handle: Option<FreeWorkerHandle<'a>>,
}

impl WorkerResponses<'_> {
    /// Returns next response.
    /// If returned response is JudgeDone or error, must not be polled again.
    pub(crate) async fn next(&mut self) -> anyhow::Result<Response> {
        let handle = self
            .handle
            .as_ref()
            .expect("WorkerResponses is polled after finish");
        let res = handle.worker.recv().await;
        let is_eos = match &res {
            Ok(ok) => matches!(ok, Response::JudgeDone(_)),
            Err(_) => true,
        };
        if is_eos {
            if res.is_ok() {
                handle.worker.state.store(WorkerStateKind::Locked);
            } else {
                handle.worker.state.store(WorkerStateKind::Crash);
            }
            self.handle.take();
        }
        res
    }
}

/// Contains WorkerStateKind
struct WorkerState(AtomicU8);
const WORKER_STATE_IDLE: u8 = 0;
const WORKER_STATE_LOCKED: u8 = 1;
const WORKER_STATE_CRASH: u8 = 2;
const WORKER_STATE_JUDGE: u8 = 3;
impl WorkerState {
    fn new(kind: WorkerStateKind) -> Self {
        let this = WorkerState(AtomicU8::new(0));
        this.store(kind);
        this
    }

    fn store(&self, kind: WorkerStateKind) {
        let value = match kind {
            WorkerStateKind::Idle => WORKER_STATE_IDLE,
            WorkerStateKind::Locked => WORKER_STATE_LOCKED,
            WorkerStateKind::Crash => WORKER_STATE_CRASH,
            WorkerStateKind::Judge => WORKER_STATE_JUDGE,
        };
        self.0.store(value, SeqCst);
    }

    fn load(&self) -> WorkerStateKind {
        let value = self.0.load(SeqCst);
        match value {
            WORKER_STATE_IDLE => WorkerStateKind::Idle,
            WORKER_STATE_LOCKED => WorkerStateKind::Locked,
            WORKER_STATE_CRASH => WorkerStateKind::Crash,
            WORKER_STATE_JUDGE => WorkerStateKind::Judge,
            other => unreachable!("unexpected worker state {}", other),
        }
    }

    /// Tries to atomically lock this worker state.
    /// I.e., this functions succeeds if state was `Idle` and it was
    /// successfully CASed to `Locked`.
    fn lock(&self) -> bool {
        self.0
            .compare_and_swap(WORKER_STATE_IDLE, WORKER_STATE_LOCKED, SeqCst)
            == WORKER_STATE_IDLE
    }
}

enum WorkerStateKind {
    /// Worker is ready for new tasks
    Idle,
    /// Worker is ready, but it is locked by a WorkerHandle
    Locked,
    /// Worker is juding run
    Judge,
    /// Worker has crashed
    Crash,
}

struct WorkerInfo {
    state: WorkerState,
    child_stdout: Mutex<tokio::io::BufReader<tokio::process::ChildStdout>>,
    child_stdin: Mutex<tokio::process::ChildStdin>,
}

impl WorkerInfo {
    pub async fn recv(&self) -> anyhow::Result<Response> {
        let mut line = String::new();
        let mut child_stdout = self.child_stdout.lock().await;

        child_stdout.read_line(&mut line).await?;
        Ok(serde_json::from_str(&line).context("parse error")?)
    }

    pub async fn send(&self, req: Request) -> anyhow::Result<()> {
        let mut data = serde_json::to_vec(&req)?;
        data.push(b'\n');
        self.child_stdin.lock().await.write_all(&data).await?;
        Ok(())
    }

    /// If this worker is idle, returns a handle to it.
    /// Otherwise, returns None
    pub fn try_lock(&self, notify: multiwake::Sender) -> Option<FreeWorkerHandle> {
        if self.state.lock() {
            Some(FreeWorkerHandle {
                worker: self,
                notify,
            })
        } else {
            None
        }
    }
}
