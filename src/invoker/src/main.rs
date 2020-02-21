use anyhow::{bail, Context};
use slog_scope::debug;
use std::sync::Arc;

fn check_system() -> anyhow::Result<()> {
    if let Some(err) = minion::check() {
        bail!("invoker is not able to test runs: {}", err);
    }
    Ok(())
}

fn install_color_backtrace() {
    #[cfg(feature = "beautiful_backtrace")]
    color_backtrace::install();
}

fn is_cli_mode() -> bool {
    std::env::args().count() > 1
}

fn make_sources(
    config: Arc<cfg::Config>,
) -> anyhow::Result<Vec<Box<dyn invoker::controller::TaskSource>>> {
    let mut sources: Vec<Box<dyn invoker::controller::TaskSource>> = Vec::new();
    if is_cli_mode() {
        let source = invoker::sources::CliSource::new();
        //let driver = invoker::drivers::CliDriver::new().context("failed to setup CLI Controller Driver")?;
        sources.push(Box::new(source));
    } else {
        let db_conn = db::connect_env().context("db connection failed")?;
        let source = invoker::sources::DbSource::new(db_conn, config);
        sources.push(Box::new(source))
    }
    Ok(sources)
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    if atty::is(atty::Stream::Stderr) {
        install_color_backtrace();
    }
    util::log::setup();
    util::wait::wait();

    let config = Arc::new(cfg::get_config());

    check_system().context("system configuration problem")?;
    debug!("system check passed");

    let backend = minion::setup();
    let driver = make_sources(config.clone()).context("failed to initialize driver")?;
    let controller = invoker::controller::Controller::new(driver, backend.into(), config, 3)
        .context("failed to start controller")?;

    util::daemon_notify_ready();
    controller.run_forever();
    Ok(())
}
