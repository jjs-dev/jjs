use anyhow::Context;
use frontend_engine::FrontendParams;
use slog_scope::info;
use std::{process::exit, sync::Arc};

fn launch_api(
    frcfg: Arc<FrontendParams>,
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
    .context("failed to create ApiServer")?
    .launch();

    slog_scope::crit!("launch error: {}", launch_err);
    exit(1)
}

fn launch_root_login_server(params: Arc<FrontendParams>) {
    let cfg = frontend_engine::root_auth::Config {
        socket_path: params.cfg.unix_socket_path.clone(),
    };
    frontend_engine::root_auth::LocalAuthServer::start(cfg, params);
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    util::log::setup();
    util::wait::wait();
    // private api
    // if you want schema, you can find it in distribuition dir
    match std::env::var("__JJS_SPEC").ok().as_deref() {
        Some("config-schema") => {
            let schema = schemars::schema_for!(frontend_engine::config::FrontendConfig);
            let mut schema =
                serde_json::to_value(&schema).expect("failed to serialize config JsonSchema");
            schema["$id"] = serde_json::Value::String("jjs.ns/frontend-config".to_string());
            let schema =
                serde_json::to_string_pretty(&schema).expect("failed to stringgify JsonSchema");
            println!("{}", schema);
            return Ok(());
        }
        Some(other) => panic!("unknown __JJS_SPEC request: {}", other),
        None => (),
    }
    let cfg_data = util::cfg::load_cfg_data().context("failed to load configuration")?;
    let raw_config = frontend_engine::config::FrontendConfig::obtain(&cfg_data.data_dir)
        .context("failed to load frontend config")?;
    let frontend_cfg = raw_config
        .into_frontend_params()
        .context("failed to create FrontendParams")?;
    let frontend_cfg = Arc::new(frontend_cfg);
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
