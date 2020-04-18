#![feature(never_type)]

use anyhow::Context;
use log::debug;
use std::sync::Arc;

fn is_cli_mode() -> bool {
    std::env::args().count() > 1
}

async fn make_sources(
    cfg_data: &util::cfg::CfgData,
    background_source_manager: invoker::sources::BackgroundSourceManager,
) -> anyhow::Result<Vec<Arc<dyn invoker::controller::TaskSource>>> {
    let mut sources: Vec<Arc<dyn invoker::controller::TaskSource>> = Vec::new();
    if is_cli_mode() {
        invoker::sources::cli_source::start(background_source_manager.fork().await);
    } else {
        let db_conn = db::connect_env().await.context("db connection failed")?;
        let source = invoker::sources::DbSource::new(db_conn, cfg_data);
        sources.push(Arc::new(source))
    }
    sources.push(Arc::new(background_source_manager.into_source()));
    Ok(sources)
}

fn worker_self_isolate() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        // TODO: unshare NEWNET too. To achieve it, we have to switch to multiprocessing instead of multithreading
        nix::sched::unshare(nix::sched::CloneFlags::CLONE_FILES).context("failed to unshare")?;
    }
    Ok(())
}

fn is_worker() -> bool {
    std::env::var("__JJS_WORKER").is_ok()
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    util::log::setup();
    if is_worker() {
        invoker::init::init().context("failed to initialize")?;
        worker_self_isolate()?;
    } else {
        util::wait::wait();
    }
    let mut rt = tokio::runtime::Builder::new();
    rt.basic_scheduler();
    let mut rt = rt.enable_all().core_threads(1).max_threads(2).build()?;
    rt.block_on(real_main())
}

async fn start_controller(
    cfg: invoker::config::InvokerConfig,
    stop_token: tokio::sync::broadcast::Receiver<!>,
    system_config_data: util::cfg::CfgData,
    background_source: invoker::sources::BackgroundSourceManager,
) -> anyhow::Result<()> {
    let driver = make_sources(&system_config_data, background_source)
        .await
        .context("failed to initialize driver")?;

    let controller = invoker::controller::Controller::new(driver, system_config_data, cfg)
        .context("failed to start controller")?;
    tokio::task::spawn(controller.run_forever(stop_token));
    Ok(())
}

async fn real_main() -> anyhow::Result<()> {
    if is_worker() {
        return invoker::worker::main().await;
    }

    let system_config_data = util::cfg::load_cfg_data()?;

    debug!("system check passed");

    let invoker_config_file_path = system_config_data.data_dir.join("etc/invoker.yaml");
    let invoker_config_data = tokio::fs::read(&invoker_config_file_path)
        .await
        .with_context(|| {
            format!(
                "unable to read config from {}",
                invoker_config_file_path.display()
            )
        })?;
    let invoker_config: invoker::config::InvokerConfig =
        serde_yaml::from_slice(&invoker_config_data).context("config parse error")?;
    let (invoker_stop_token, invoker_stop_token_rx) = tokio::sync::broadcast::channel(1);
    // TODO probably broken for IPv6
    let bind_address = format!("{}:{}", invoker_config.api.address, invoker_config.api.port);
    let bind_address = bind_address
        .parse()
        .with_context(|| format!("invalid bind address {}", bind_address))?;

    let bg_source = invoker::sources::BackgroundSourceManager::create();

    let (mut shutdown_trigger_tx, mut shutdown_trigger_rx) = tokio::sync::mpsc::channel(1);
    invoker::api::start(
        invoker_stop_token.subscribe(),
        bind_address,
        bg_source.fork().await,
        shutdown_trigger_tx.clone(),
        system_config_data.data_dir.join("etc/pki"),
    )
    .await
    .context("failed to start api")?;
    start_controller(
        invoker_config,
        invoker_stop_token_rx,
        system_config_data,
        bg_source,
    )
    .await
    .context("can not start controller")?;

    util::daemon_notify_ready();
    tokio::task::spawn(async move {
        log::debug!("Installing signal hook");
        match tokio::signal::ctrl_c().await {
            Ok(_) => {
                log::info!("Received ctrl-c");
                shutdown_trigger_tx.send(()).await.ok();
            }
            Err(err) => log::warn!("Failed to wait for signal: {}", err),
        }
    });
    shutdown_trigger_rx.recv().await;
    log::info!("Received shutdown request; exiting gracefully");
    drop(invoker_stop_token);
    Ok(())
}
