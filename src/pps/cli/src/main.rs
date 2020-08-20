#![feature(is_sorted)]
#![allow(clippy::needless_lifetimes)]

mod client_util;
mod compile;
mod import;
mod progress_notifier;

#[derive(clap::Clap, Debug)]
#[clap(author, about)]
pub enum Args {
    Compile(compile::CompileArgs),
    Import(import::ImportArgs),
}

use anyhow::Context as _;
use std::path::Path;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Clap;
    util::log::setup();
    let args = Args::parse();
    tracing::info!("starting new server in background");
    let cancel = tokio::sync::CancellationToken::new();
    let (server_done_rx, mut client) = client_util::create_server(cancel.clone()).await?;
    process_args(args, &mut client)
        .await
        .context("failed to process args")?;
    cancel.cancel();
    tracing::info!("waiting for server shutdown");
    server_done_rx.await.ok();
    Ok(())
}

#[tracing::instrument(skip(args, client))]
async fn process_args(args: Args, client: &mut rpc::Client) -> anyhow::Result<()> {
    tracing::info!(args=?args, "executing requested command");
    match args {
        Args::Compile(compile_args) => compile::exec(client, compile_args).await,
        Args::Import(import_args) => import::exec(client, import_args).await,
    }
}
