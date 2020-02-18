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

fn make_driver(
    config: Arc<cfg::Config>,
) -> anyhow::Result<Box<dyn invoker::controller::ControllerDriver>> {
    if is_cli_mode() {
        let driver = std::sync::Arc::new(invoker::drivers::SillyDriver::new());
        invoker::drivers::enable_cli(driver.clone());
        //let driver = invoker::drivers::CliDriver::new().context("failed to setup CLI Controller Driver")?;
        return Ok(Box::new(driver));
    }
    let db_conn = db::connect_env().context("db connection failed")?;
    let driver = invoker::drivers::DbDriver::new(db_conn, config);
    Ok(Box::new(driver))
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
    let driver = make_driver(config.clone()).context("failed to initialize driver")?;
    let controller = invoker::controller::Controller::new(driver, backend.into(), config, 3)
        .context("failed to start controller")?;

    util::daemon_notify_ready();
    controller.run_forever();
    Ok(())
}
