use anyhow::Context;
use slog_scope::info;
use std::process::exit;

use frontend_engine::FrontendConfig;

fn launch_api(
    frcfg: FrontendConfig,
    entity_loader: entity::Loader,
    problem_loader: problem_loader::Loader,
    data_dir: &std::path::Path,
) -> anyhow::Result<()> {
    let pool = db::connect_env().context("DB connection failed")?;

    let launch_err = frontend_engine::ApiServer::create(
        frcfg,
        entity_loader,
        pool.into(),
        problem_loader,
        data_dir,
    )
    .launch();

    slog_scope::crit!("launch error: {}", launch_err);
    exit(1)
}

fn launch_root_login_server(fcfg: FrontendConfig) {
    let cfg = frontend_engine::root_auth::Config {
        socket_path: String::from("/tmp/jjs-auth-sock"), /* TODO dehardcode */
    };
    frontend_engine::root_auth::LocalAuthServer::start(cfg, &fcfg);
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    util::log::setup();
    util::wait::wait();
    let frontend_cfg = frontend_engine::config::FrontendConfig::obtain();
    let cfg_data = util::cfg::load_cfg_data().context("failed to load configuration")?;
    info!("starting frontend");

    launch_root_login_server(frontend_cfg.clone());
    util::daemon_notify_ready();
    launch_api(
        frontend_cfg,
        cfg_data.entity_loader,
        cfg_data.problem_loader,
        &cfg_data.data_dir,
    )
    .context("failed to start api service")?;
    Ok(())
}
