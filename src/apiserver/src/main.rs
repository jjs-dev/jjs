use anyhow::Context;
use futures::stream::StreamExt;
use log::info;

async fn launch_api(
    config: apiserver_engine::config::ApiserverConfig,
    entity_loader: entity::Loader,
    problem_loader: problem_loader::Loader,
    data_dir: std::path::PathBuf,
    db_conn: db::DbConn,
) -> anyhow::Result<apiserver_engine::ShutdownHandle> {
    let token_manager = {
        let db_conn = db_conn.clone();
        let secret_key = apiserver_engine::config::read_secret_from_env(config.env.is_prod());
        apiserver_engine::TokenMgr::new(db_conn, secret_key.into())
    };
    let params = apiserver_engine::ApiserverParams {
        token_manager,
        config,
        entity_loader,
        problem_loader,
        data_dir,
        db_conn,
        tls_mode: apiserver_engine::TlsMode::Enabled,
    };
    let server = apiserver_engine::ApiServer::create(params)
        .await
        .context("failed to create ApiServer")?;
    let shutdown = server.get_shutdown_handle().clone();

    Ok(shutdown)
}

async fn launch_root_login_server(
    db_conn: db::DbConn,
    config: apiserver_engine::config::ApiserverConfig,
    close: tokio::sync::oneshot::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    let cfg = apiserver_engine::root_auth::Config {
        socket_path: config.unix_socket_path.clone(),
    };
    let token_manager = apiserver_engine::TokenMgr::new(
        db_conn,
        apiserver_engine::config::read_secret_from_env(config.env.is_prod()).into(),
    );

    tokio::task::spawn(async move {
        apiserver_engine::root_auth::exec(cfg, token_manager, close).await;
    })
}

async fn should_shutdown() -> anyhow::Result<()> {
    let sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    let mut joined = futures::stream::select(sigint, sigterm);
    if joined.next().await.is_none() {
        loop {
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    util::log::setup();
    daemons::wait::wait();
    // private api
    // if you want schema, you can find it in distribuition dir
    match std::env::var("__JJS_SPEC").ok().as_deref() {
        Some("config-schema") => {
            let schema = schemars::schema_for!(apiserver_engine::config::ApiserverConfig);
            let schema =
                serde_json::to_value(&schema).expect("failed to serialize config JsonSchema");
            let schema =
                serde_json::to_string_pretty(&schema).expect("failed to stringify JsonSchema");
            println!("{}", schema);
            return Ok(());
        }
        Some("api-models") => {
            let data = apiserver_engine::introspect::introspect();
            let data = serde_json::to_string(&data).expect("failed to serialize API models");
            println!("{}", data);
            return Ok(());
        }
        Some(other) => panic!("unknown __JJS_SPEC request: {}", other),
        None => (),
    }
    let cfg_data = daemons::cfg::load_cfg_data().context("failed to load configuration")?;
    let apiserver_cfg = apiserver_engine::config::ApiserverConfig::obtain(&cfg_data.data_dir)
        .context("failed to load apiserver config")?;
    info!("starting apiserver");

    let (login_send, login_recv) = tokio::sync::oneshot::channel();

    let db_conn = db::connect_env().await.context("DB connection failed")?;

    let login_join =
        launch_root_login_server(db_conn.clone(), apiserver_cfg.clone(), login_recv).await;
    daemons::daemon_notify_ready();
    let api_shutdown = launch_api(
        apiserver_cfg,
        cfg_data.entity_loader,
        cfg_data.problem_loader,
        cfg_data.data_dir.clone(),
        db_conn,
    )
    .await
    .context("failed to start api service")?;
    should_shutdown().await?;
    login_send
        .send(())
        .map_err(|_unit| anyhow::anyhow!("Failed to shutdown login service"))?;
    match tokio::time::timeout(std::time::Duration::from_secs(1), login_join).await {
        Ok(ret) => ret?,
        Err(_elapsed) => anyhow::bail!("Timeout waiting for login service shutdown"),
    }
    api_shutdown.shutdown();

    Ok(())
}
