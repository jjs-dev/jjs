#![feature(proc_macro_hygiene, decl_macro, param_attrs)]

extern crate rocket;

use slog::{error, Logger};
use std::process::exit;

use frontend_engine::FrontendConfig;

fn launch_api(frcfg: FrontendConfig, logger: Logger, config: cfg::Config) {
    let pool = match db::connect_env() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("failed connect to DB: {}", e);
            exit(1);
        }
    };

    let launch_err =
        frontend_engine::ApiServer::create(frcfg, logger.clone(), &config, pool.into()).launch();

    error!(logger, "launch error: {}", launch_err);
    exit(1)
}

fn launch_root_login_server(logger: &slog::Logger, fcfg: FrontendConfig) {
    let cfg = frontend_engine::root_auth::Config {
        socket_path: String::from("/tmp/jjs-auth-sock"), /* FIXME dehardcode */
    };
    let sublogger = logger.new(slog::o!("app" => "jjs:frontend:localauth"));
    frontend_engine::root_auth::LocalAuthServer::start(sublogger, cfg.clone(), &fcfg);
}

fn main() {
    use slog::Drain;
    dotenv::dotenv().ok();
    let frontend_cfg = frontend_engine::config::FrontendConfig::obtain();
    let cfg = cfg::get_config();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, slog::o!("app" => "jjs:frontend"));
    slog::info!(logger, "starting frontend");

    launch_root_login_server(&logger, frontend_cfg.clone());
    launch_api(frontend_cfg, logger, cfg);
}
