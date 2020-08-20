#![feature(is_sorted)]
#![allow(clippy::needless_lifetimes)]

mod command;
mod compile;
mod import;
mod manifest;

use anyhow::Context;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};

fn check_dir(path: &Path, allow_nonempty: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("error: path {} not exists", path.display());
    }
    if !path.is_dir() {
        anyhow::bail!("error: path {} is not directory", path.display());
    }
    if !allow_nonempty && path.read_dir().unwrap().next().is_some() {
        anyhow::bail!("error: dir {} is not empty", path.display());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[tracing::instrument]
fn tune_linux() -> anyhow::Result<()> {
    let mut current_limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        if libc::prlimit(0, libc::RLIMIT_STACK, std::ptr::null(), &mut current_limit) != 0 {
            anyhow::bail!("get current RLIMIT_STACK");
        }
    }
    let new_limit = libc::rlimit {
        rlim_cur: current_limit.rlim_max,
        rlim_max: current_limit.rlim_max,
    };
    unsafe {
        if libc::prlimit(0, libc::RLIMIT_STACK, &new_limit, std::ptr::null_mut()) != 0 {
            anyhow::bail!("update RLIMIT_STACK");
        }
    }

    Ok(())
}

#[tracing::instrument]
fn tune_resource_limits() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    tune_linux()?;

    Ok(())
}

/// Returns `rpc::Router` with all PPS api routes installed.
pub async fn create_server() -> anyhow::Result<rpc::Router> {
    let mut builder = rpc::RouterBuilder::new();

    let service = Service(Arc::new(ServiceState::get().await?));
    builder.add_route::<pps_api::CompileProblem, _>(service.clone());
    builder.add_route::<pps_api::ImportProblem, _>(service);
    Ok(builder.build())
}

/// Starts PPS server on specified port on background tokio task.
#[tracing::instrument(skip(cancel))]
pub async fn serve(
    port: u16,
    cancel: tokio::sync::CancellationToken,
) -> anyhow::Result<tokio::sync::oneshot::Receiver<()>> {
    tune_resource_limits()?;

    let router = create_server().await?;

    let bind_addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let server = hyper::Server::try_bind(&bind_addr)?
        .serve(router.as_make_service())
        .with_graceful_shutdown(async move { cancel.cancelled().await });
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn(async move {
        server.await.expect("serve error");
        tx.send(()).ok();
    });
    Ok(rx)
}

#[derive(Clone)]
pub struct Service(pub(crate) Arc<ServiceState>);

pub struct ServiceState {
    /// JJS installation directory (used to find JTL binaries)
    jjs_dir: PathBuf,
}

impl ServiceState {
    pub async fn get() -> anyhow::Result<ServiceState> {
        let jjs_dir: PathBuf = std::env::var_os("JJS_PATH")
            .context("JJS_PATH not set")?
            .into();
        Ok(ServiceState { jjs_dir })
    }
}
