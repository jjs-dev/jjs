mod channel;

/// Implements `InvokerSet`
use crate::config::JudgeConfig;
use anyhow::Context as _;
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc};
use tracing::{debug, instrument};
/// InvokerSet manages internal invoker and connects to external.
#[derive(Clone)]
pub struct InvokerSet {
    /// Information abount spawned invokers
    managed: Arc<[Arc<WorkerInfo>]>,
    /// these field is used to signal that a worker is reclaimed
    worker_reclamation: Arc<event_listener::Event>,
}

pub struct InvokerSetBuilder {
    /// Path to invoker binary
    invoker_path: PathBuf,
    /// workers spawned so far
    managed: Vec<Arc<WorkerInfo>>,
}

impl InvokerSetBuilder {
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
        // we use 1 as capacity because in fact we never send request to invoker before it
        // responded to previous
        let (req_tx, req_rx) = async_channel::bounded(1);
        let (res_tx, res_rx) = async_channel::bounded(1);
        tokio::task::spawn(channel::serve(
            child.stdin.take().expect("child stdin was captured"),
            child.stdout.take().expect("child stdout was captured"),
            req_rx,
            res_tx,
        ));
        let info = WorkerInfo {
            state: Mutex::new(WorkerState::Idle),
            send_request: req_tx,
            recv_response: res_rx,
        };
        self.managed.push(Arc::new(info));
        Ok(())
    }

    /// Finalizes InvokerSet construction
    pub fn build(self) -> InvokerSet {
        InvokerSet {
            managed: self.managed.into(),
            worker_reclamation: Arc::new(event_listener::Event::new()),
        }
    }
}

impl InvokerSet {
    /// Creates new builder
    pub fn builder(config: &JudgeConfig) -> InvokerSetBuilder {
        InvokerSetBuilder {
            invoker_path: config.invoker_path,
            managed: Vec::new(),
        }
    }

    /// Finds a free worker (waiting if needed) and sends http request.
    #[instrument(skip(self, req))]
    async fn send_request(&self, req: hyper::Request<hyper::Body>) -> hyper::Response<hyper::Body> {
        let mut attempt_id = 0u32;
        loop {
            debug!(attempt_id, "scanning all workers");
            attempt_id += 1;
            let worker_reclaimed = self.worker_reclamation.listen();
            for worker in &*self.managed {
                if let Some(handle) = worker.try_lock(self.worker_reclamation.clone()) {
                    handle.send_request.send(req).await.expect("worker died");
                    let resp = handle
                        .recv_response
                        .recv()
                        .await
                        .expect("worker died before responding");
                    return resp;
                }
            }

            worker_reclaimed.await;
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
struct WorkerInfo {
    // could be AtomicU8, but mutex is simpler
    state: Mutex<WorkerState>,
    // Danger: must not be used concurrently, otherwise
    // we can receive wrong response
    send_request: async_channel::Sender<hyper::Request<hyper::Body>>,
    recv_response: async_channel::Receiver<hyper::Response<hyper::Body>>,
}
struct LockedWorker {
    send_request: async_channel::Sender<hyper::Request<hyper::Body>>,
    recv_response: async_channel::Receiver<hyper::Response<hyper::Body>>,
    notify_on_drop: Arc<event_listener::Event>,
    worker: Arc<WorkerInfo>,
}

impl LockedWorker {
    async fn call(self, req: hyper::Request<hyper::Body>) -> hyper::Response<hyper::Body> {
        let wake = {
            let ev = self.notify_on_drop.clone();
            move || {
                ev.notify_additional(1);
            }
        };

        let result = async move {
            self.send_request
                .send(req)
                .await
                .expect("unexpected contention");

            self.recv_response
                .recv()
                .await
                .expect("response should be sent and non-stolen")
        }
        .await;
        wake();
        result
    }
}

impl Drop for LockedWorker {
    fn drop(&mut self) {
        // mark Worker as idle
        *self.worker.state.lock() = WorkerState::Idle;
        // trigger event
        self.notify_on_drop.notify_additional(1);
    }
}

impl WorkerInfo {
    fn try_lock(
        self: &Arc<Self>,
        notify_on_drop: Arc<event_listener::Event>,
    ) -> Option<LockedWorker> {
        let mut lock = self.state.lock();
        if *lock != WorkerState::Idle {
            return None;
        }
        *lock = WorkerState::Locked;
        Some(LockedWorker {
            send_request: self.send_request.clone(),
            recv_response: self.recv_response.clone(),
            worker: self.clone(),
            notify_on_drop,
        })
    }
}

impl hyper::service::Service<hyper::Request<hyper::Body>> for InvokerSet {
    type Error = std::convert::Infallible;
    type Future = futures_util::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = hyper::Response<hyper::Body>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: hyper::Request<hyper::Body>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move { Ok(this.send_request(req).await) })
    }
}
