use slog_scope::{error, info};
use std::process::exit;

use frontend_engine::FrontendConfig;

fn launch_api(frcfg: FrontendConfig, config: cfg::Config) {
    let pool = match db::connect_env() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("failed connect to DB: {}", e);
            exit(1);
        }
    };

    let launch_err = frontend_engine::ApiServer::create(frcfg, &config, pool.into()).launch();

    error!("launch error: {}", launch_err);
    exit(1)
}

fn launch_root_login_server(fcfg: FrontendConfig) {
    let cfg = frontend_engine::root_auth::Config {
        socket_path: String::from("/tmp/jjs-auth-sock"), /* TODO dehardcode */
    };
    frontend_engine::root_auth::LocalAuthServer::start(cfg, &fcfg);
}

fn main() {
    dotenv::dotenv().ok();
    util::log::setup();
    util::wait::wait();
    let frontend_cfg = frontend_engine::config::FrontendConfig::obtain();
    let cfg = cfg::get_config();
    info!("starting frontend");

    launch_root_login_server(frontend_cfg.clone());
    util::daemon_notify_ready();
    launch_api(frontend_cfg, cfg);
}
