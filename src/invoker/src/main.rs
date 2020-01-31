mod controller;
mod drivers;
mod worker;

use anyhow::{bail, Context};
use slog_scope::debug;

fn check_system() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        if let Some(err) = minion::linux_check_environment() {
            bail!("invoker is not able to test runs: {}", err);
        }
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

fn make_driver() -> anyhow::Result<Box<dyn controller::ControllerDriver>> {
    if is_cli_mode() {
        let driver = drivers::CliDriver::new().context("failed to setup CLI Controller Driver")?;
        return Ok(Box::new(driver));
    }
    let db_conn = db::connect_env().context("db connection failed")?;
    let driver = drivers::DbDriver::new(db_conn);
    Ok(Box::new(driver))
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    if atty::is(atty::Stream::Stderr) {
        install_color_backtrace();
    }
    util::log::setup();
    util::wait::wait();

    let config = cfg::get_config();

    check_system().context("system configuration problem")?;
    debug!("system check passed");

    let backend = minion::setup();
    let driver = make_driver().context("failed to initialize driver")?;
    let controller = controller::Controller::new(driver, backend.into(), config, 3)
        .context("failed to start controller")?;

    util::daemon_notify_ready();
    controller.run_forever();
    Ok(())
}
