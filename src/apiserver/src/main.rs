use anyhow::Context;
use apiserver_engine::ApiserverParams;
use futures::stream::StreamExt;
use log::info;
use std::sync::Arc;

async fn launch_api(
    frcfg: Arc<ApiserverParams>,
    entity_loader: entity::Loader,
    problem_loader: problem_loader::Loader,
    data_dir: &std::path::Path,
    cancel_token: tokio::sync::CancellationToken,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    let pool = db::connect_env().await.context("DB connection failed")?;
    let rocket = apiserver_engine::ApiServer::create(
        frcfg,
        entity_loader,
        pool.into(),
        problem_loader,
        data_dir,
    )
    .context("failed to create ApiServer")?
    .take_rocket();
    let shutdown = rocket.get_shutdown_handle();
    tokio::task::spawn(async move {
        cancel_token.cancelled().await;
        shutdown.shutdown();
    });
    let launch_fut = rocket.serve();

    let join = tokio::task::spawn(async {
        if let Err(err) = launch_fut.await {
            log::error!("Serve error: {}", err);
        }
    });

    Ok(join)
}

async fn launch_root_login_server(
    params: Arc<ApiserverParams>,
    cancel_token: tokio::sync::CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let cfg = apiserver_engine::root_auth::Config {
        socket_path: params.cfg.unix_socket_path.clone(),
    };
    tokio::task::spawn(async {
        apiserver_engine::root_auth::exec(cfg, params, cancel_token).await;
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
    util::wait::wait();
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
    let cfg_data = util::cfg::load_cfg_data().context("failed to load configuration")?;
    let raw_config = apiserver_engine::config::ApiserverConfig::obtain(&cfg_data.data_dir)
        .context("failed to load apiserver config")?;
    let apiserver_cfg = raw_config
        .into_apiserver_params()
        .await
        .context("failed to create ApiserverParams")?;
    let apiserver_cfg = Arc::new(apiserver_cfg);
    info!("starting apiserver");

    let cancel_token = tokio::sync::CancellationToken::new();

    let login_join = launch_root_login_server(apiserver_cfg.clone(), cancel_token.clone()).await;
    util::daemon_notify_ready();
    let api_join = launch_api(
        apiserver_cfg,
        cfg_data.entity_loader,
        cfg_data.problem_loader,
        &cfg_data.data_dir,
        cancel_token.clone(),
    )
    .await
    .context("failed to start api service")?;

    should_shutdown().await?;

    info!("Shutdown was requested");

    cancel_token.cancel();

    match tokio::time::timeout(std::time::Duration::from_secs(1), login_join).await {
        Ok(ret) => ret?,
        Err(_elapsed) => anyhow::bail!("Timeout waiting for login service shutdown"),
    }

    match tokio::time::timeout(std::time::Duration::from_secs(15), api_join).await {
        Ok(ret) => ret?,
        Err(_elapsed) => anyhow::bail!("Timeout waiting for API service shutdown"),
    }

    Ok(())
}
