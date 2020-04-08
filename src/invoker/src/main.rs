use anyhow::Context;
use log::debug;
use std::sync::Arc;

fn is_cli_mode() -> bool {
    std::env::args().count() > 1
}

async fn make_sources(
    cfg_data: &util::cfg::CfgData,
) -> anyhow::Result<Vec<Arc<dyn invoker::controller::TaskSource>>> {
    let mut sources: Vec<Arc<dyn invoker::controller::TaskSource>> = Vec::new();
    if is_cli_mode() {
        let source = invoker::sources::CliSource::new();
        sources.push(Arc::new(source));
    } else {
        let db_conn = db::connect_env().await.context("db connection failed")?;
        let source = invoker::sources::DbSource::new(db_conn, cfg_data);
        sources.push(Arc::new(source))
    }
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
    if is_worker() {
        rt.basic_scheduler();
    } else {
        rt.threaded_scheduler();
    }
    let mut rt = rt.enable_all().core_threads(1).max_threads(2).build()?;
    rt.block_on(async { tokio::task::spawn(real_main()).await.unwrap() })
}

async fn real_main() -> anyhow::Result<()> {
    if is_worker() {
        return invoker::worker::main().await;
    }

    let config = util::cfg::load_cfg_data()?;

    debug!("system check passed");

    let driver = make_sources(&config)
        .await
        .context("failed to initialize driver")?;
    let controller = invoker::controller::Controller::new(driver, config, 3)
        .context("failed to start controller")?;

    util::daemon_notify_ready();
    controller.run_forever().await;
    Ok(())
}
