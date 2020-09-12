/// Implements `InvokerSet`
use crate::config::JudgeConfig;
use anyhow::Context as _;
use futures_util::{
    future::{FutureExt, TryFutureExt},
    stream::StreamExt,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU8, Ordering::SeqCst},
        Arc,
    },
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tracing::{debug, instrument};
/// InvokerSet manages internal invoker and connects to external.
pub struct InvokerSet {
    /// Information abount spawned invokers
    managed: Vec<WorkerInfo>,
    /// Path to invoker binary
    invoker_path: PathBuf,
    /// these field is used to signal that a worker is reclaimed
    worker_reclamation: event_listener::Event,
}

impl InvokerSet {
    /// Creates new Invoker with empty `managed` set
    pub fn new(config: &JudgeConfig) -> anyhow::Result<Self> {
        Ok(InvokerSet {
            managed: vec![],
            invoker_path: config.invoker_path,
            worker_reclamation: event_listener::Event::new(),
        })
    }

    /// Starts new invoker process and adds it to this InvokerSet
    #[instrument(skip(self))]
    pub async fn add_managed_worker(&mut self) -> anyhow::Result<()> {
        let mut child = tokio::process::Command::new(&self.invoker_path)
            .arg("serve")
            .arg("--address=cli")
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
        self.managed.push(info);
        Ok(())
    }

    /// Finds a free invoker (waiting if needed) and executes given InvokeRequest.
    #[instrument(skip(self, req))]
    pub async fn send_request(
        &self,
        req: judging_apis::invoke::InvokeRequest,
    ) -> anyhow::Result<judging_apis::invoke::InvokeResponse> {
        let mut attempt_id = 0u32;
        loop {
            debug!(attempt_id, "scanning all workers");
            attempt_id += 1;
            let mut worker_reclaimed = self.worker_reclamation.listen();
            for worker in &self.managed {
                if let Some(handle) = worker.try_lock(self.worker_reclamation.0.clone()) {
                    return handle;
                }
            }
            worker_reclaimed.await;
        }
    }
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

#[derive(Eq, PartialEq)]
enum WorkerState {
    /// Worker is ready for new tasks
    Idle,
    /// Worker is ready, but it is locked by a WorkerHandle
    Locked,
    /// Worker is juding run
    Judge,
    /// Worker has crashed
    Crash,
}

struct WorkerDataInner {
    io: std::sync::Mutex<(
        tokio::io::BufWriter<tokio::process::ChildStdin>,
        tokio::io::BufReader<tokio::process::ChildStdout>,
    )>,
    // could be AtomicU8, but mutex is simpler
    state: std::sync::Mutex<WorkerState>,
    request_done: event_listener::Event,
}

/// Implements "http client" on top of child stdio
struct WorkerData {
    inner: Arc<WorkerDataInner>,
}

impl WorkerData {
    fn subscribe(&self) -> event_listener::EventListener {
        self.inner.request_done.listen()
    }

    fn call(
        &mut self,
        req: hyper::Request<hyper::Body>,
    ) -> Option<
        futures_util::future::BoxFuture<'static, anyhow::Result<hyper::Response<hyper::Body>>>,
    > {
        let mut lock = self.inner.state.lock().unwrap();
        if *lock != WorkerState::Idle {
            return None;
        }
        *lock = WorkerState::Locked;
        drop(lock);
        let wake = {
            let inner = self.inner.clone();
            move |_future_res: &anyhow::Result<hyper::Response<hyper::Body>>| {
                inner.request_done.notify_additional(1);
            }
        };
        let inner = self.inner.clone();
        let mut io = inner.io.lock().unwrap();
        let (stdin, stdout) = &mut *io;

        Some(
            (async move {
                let mut uri = req.uri().to_string();
                uri.push('\n');
                // tokio::pin!(stdout);
                // tokio::pin!(stdin);
                stdin.write_all(uri.as_bytes()).await?;
                let req_body = hyper::body::to_bytes(req.into_body()).await?;
                let cnt = req_body.len();
                stdin.write_all(&cnt.to_ne_bytes()).await?;
                stdin.write_all(&req_body).await?;
                stdin.flush().await?;

                Result::<hyper::Response<hyper::Body>, anyhow::Error>::Ok(todo!())
            })
            .inspect_ok(wake)
            .boxed(),
        )
    }
}

impl tower_service::Service<hyper::Request<hyper::Body>> for InvokerSet {
    type Error = anyhow::Error;
    type Future = futures_util::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = hyper::Response<hyper::Body>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: hyper::Request<hyper::Body>) -> Self::Future {
        if Arc::strong_count(&self.inner) != 1 {
            panic!("")
        }
        self.in_flight = true;
        Box::pin(async move {})
    }
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
