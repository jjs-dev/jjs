//! This module implements compiling source package into invoker package
pub(crate) mod build;
mod builder;
mod progress_notifier;

use anyhow::Context as _;
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use tokio::sync::{mpsc, oneshot};

/// Represents single request data to CompilerService.
pub struct CompileSingleProblemArgs {
    pub pkg_path: PathBuf,
    pub out_path: PathBuf,
    pub force: bool,
}

struct ServiceRequest {
    /// Request data, provided by user
    args: CompileSingleProblemArgs,
    /// channel which should receive response.
    /// Dropping it does not cancel build.
    chan: oneshot::Sender<anyhow::Result<()>>,
}

const CHANNEL_CAPACITY: usize = 16;

/// Represents long-running background task, building problems on request.
/// When last `CompilerServiceClient` is dropped, service will stop automatically.
pub(crate) struct CompilerService {
    data: ServiceData,
    chan: mpsc::Receiver<ServiceRequest>,
    state_update_notify: multiwake::Sender,
    current_state: Arc<RwLock<ServiceState>>,
}

#[derive(Clone)]
pub(crate) struct ServiceState {
    pub(crate) service_running: bool,
    pub(crate) in_flight_requests: usize,
}

#[derive(Clone)]
struct ServiceData {
    jjs_dir: PathBuf,
}

/// Handle for interacting with CompilerService
#[derive(Clone)]
pub(crate) struct CompilerServiceClient {
    chan: Option<mpsc::Sender<ServiceRequest>>,
    state_update_notify: multiwake::Receiver,
    current_state: Arc<RwLock<ServiceState>>,
}

impl CompilerService {
    pub(crate) async fn start() -> anyhow::Result<CompilerServiceClient> {
        let jjs_dir: PathBuf = std::env::var_os("JJS_PATH")
            .context("JJS_PATH not set")?
            .into();

        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let state = Arc::new(RwLock::new(ServiceState {
            service_running: true,
            in_flight_requests: 0,
        }));
        let (state_notify_tx, state_notify_rx) = multiwake::multiwake();
        let service = CompilerService {
            chan: rx,
            data: ServiceData { jjs_dir },
            current_state: state.clone(),
            state_update_notify: state_notify_tx,
        };
        tokio::task::spawn(async move {
            if let Err(err) = service.serve().await {
                eprintln!("Serve error: {:#}", err);
            }
        });
        Ok(CompilerServiceClient {
            chan: Some(tx),
            state_update_notify: state_notify_rx,
            current_state: state,
        })
    }

    async fn serve(mut self) -> anyhow::Result<()> {
        while let Some(request) = self.chan.recv().await {
            let data = self.data.clone();
            let state = self.current_state.clone();
            let notify = self.state_update_notify.clone();
            {
                state.write().unwrap().in_flight_requests += 1;
                notify.wake();
            }
            tokio::task::spawn(async move {
                let res = Self::compile_problem(data, request.args).await;
                request.chan.send(res).ok();
                {
                    state.write().unwrap().in_flight_requests -= 1;
                    notify.wake();
                }
            });
        }

        {
            let mut state = self.current_state.write().unwrap();
            state.service_running = false;
        }
        self.state_update_notify.wake();

        Ok(())
    }

    async fn compile_problem(
        data: ServiceData,
        args: CompileSingleProblemArgs,
    ) -> anyhow::Result<()> {
        if args.force {
            std::fs::remove_dir_all(&args.out_path).ok();
            tokio::fs::create_dir_all(&args.out_path).await?;
        } else {
            crate::check_dir(&args.out_path, false /* TODO */)?;
        }
        let toplevel_manifest = args.pkg_path.join("problem.toml");
        let toplevel_manifest = std::fs::read_to_string(toplevel_manifest)?;

        let raw_problem_cfg: crate::manifest::RawProblem =
            toml::from_str(&toplevel_manifest).expect("problem.toml parse error");
        let (problem_cfg, warnings) = raw_problem_cfg.postprocess()?;

        if !warnings.is_empty() {
            eprintln!("{} warnings", warnings.len());
            for warn in warnings {
                eprintln!("- {}", warn);
            }
        }

        let out_dir = args.out_path.canonicalize().expect("resolve out dir");
        let problem_dir = args
            .pkg_path
            .canonicalize()
            .context("resolve problem dir")?;

        let builder = builder::ProblemBuilder {
            cfg: &problem_cfg,
            problem_dir: &problem_dir,
            out_dir: &out_dir,
            jtl_dir: &data.jjs_dir,
            build_backend: &build::Pibs {
                jjs_dir: Path::new(&data.jjs_dir),
            },
        };
        builder.build().await
    }
}

impl CompilerServiceClient {
    /// Asks CompilerService to compile specified problem.
    /// If this client was closed, error is returned.
    pub(crate) fn exec(
        &self,
        args: CompileSingleProblemArgs,
    ) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let chan = self.chan.clone();

        async move {
            let mut sender = match &chan {
                Some(chan) => chan.clone(),
                None => anyhow::bail!("This client is closed"),
            };
            let (done_tx, done_rx) = oneshot::channel();
            let req = ServiceRequest {
                args,
                chan: done_tx,
            };
            if sender.send(req).await.is_err() {
                anyhow::bail!("Task queue is full")
            }

            match done_rx.await {
                Ok(result) => result,
                Err(_recv_error) => anyhow::bail!("Service is crashed or killed"),
            }
        }
    }

    /// Shutdowns this client. If it was last non-closed client, service
    /// will exit. All clones of this client will be in closed state too.
    pub(crate) fn close(&mut self) {
        self.chan.take();
    }

    /// Returns current service state.
    /// If there are other clients, this state can be instantly outdated.
    pub(crate) fn state(&self) -> ServiceState {
        self.current_state.read().unwrap().clone()
    }

    /// Waits until state has changed
    pub(crate) async fn state_changed(&mut self) -> anyhow::Result<()> {
        if self.state_update_notify.wait().await == multiwake::WaitResult::Closed {
            anyhow::bail!("Service is crashed or killed")
        }
        Ok(())
    }
}
