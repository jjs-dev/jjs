use anyhow::Context;
use std::sync::Arc;

fn is_cli_mode() -> bool {
    std::env::args().count() > 1
}

async fn make_sources(
    background_source_manager: invoker::sources::BackgroundSourceManager,
) -> anyhow::Result<Vec<Arc<dyn invoker::controller::TaskSource>>> {
    let mut sources: Vec<Arc<dyn invoker::controller::TaskSource>> = Vec::new();
    if is_cli_mode() {
        invoker::sources::cli_source::start(background_source_manager.fork().await);
    } else {
        let api = client::connect().await.context("API connection failed")?;
        let source = invoker::sources::ApiSource::new(api);
        sources.push(Arc::new(source))
    }
    sources.push(Arc::new(background_source_manager.into_source()));
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
    rt.basic_scheduler();
    let mut rt = rt.enable_all().core_threads(1).max_threads(2).build()?;
    rt.block_on(real_main())
}

async fn start_controller(
    cfg: invoker::config::InvokerConfig,
    cancel_token: tokio::sync::CancellationToken,
    system_config_data: util::cfg::CfgData,
    background_source: invoker::sources::BackgroundSourceManager,
) -> anyhow::Result<()> {
    let driver = make_sources(background_source)
        .await
        .context("failed to initialize driver")?;

    let controller = invoker::controller::Controller::new(driver, system_config_data, cfg)
        .await
        .context("failed to start controller")?;
    tokio::task::spawn(controller.run_forever(cancel_token));
    Ok(())
}

async fn real_main() -> anyhow::Result<()> {
    if is_worker() {
        return invoker::worker::main().await;
    }

    let system_config_data = util::cfg::load_cfg_data()?;

    // now we should fetch InvokerConfig
    // we have generic `get_config_from_fs` and specific `get_config_from_k8s`
    let invoker_config = {
        if let Some(cfg) = get_config_from_k8s().await? {
            cfg
        } else {
            get_config_from_fs(&system_config_data).await?
        }
    };
    // TODO probably broken for IPv6
    let bind_address = format!("{}:{}", invoker_config.api.address, invoker_config.api.port);
    let bind_address = bind_address
        .parse()
        .with_context(|| format!("invalid bind address {}", bind_address))?;

    let bg_source = invoker::sources::BackgroundSourceManager::create();

    let cancel_token = tokio::sync::CancellationToken::new();

    invoker::api::start(
        cancel_token.clone(),
        bind_address,
        bg_source.fork().await,
        system_config_data.data_dir.join("etc/pki"),
    )
    .await
    .context("failed to start api")?;
    start_controller(
        invoker_config,
        cancel_token.clone(),
        system_config_data,
        bg_source,
    )
    .await
    .context("can not start controller")?;

    util::daemon_notify_ready();
    {
        let cancel_token = cancel_token.clone();
        tokio::task::spawn(async move {
            log::debug!("Installing signal hook");
            match tokio::signal::ctrl_c().await {
                Ok(_) => {
                    log::info!("Received ctrl-c");
                    cancel_token.cancel();
                }
                Err(err) => log::warn!("Failed to wait for signal: {}", err),
            }
        });
    }
    cancel_token.cancelled().await;
    log::info!("Received shutdown request; exiting gracefully");
    Ok(())
}

pub async fn get_config_from_fs(
    cfg_data: &util::cfg::CfgData,
) -> anyhow::Result<invoker::config::InvokerConfig> {
    let invoker_config_file_path = cfg_data.data_dir.join("etc/invoker.yaml");
    let invoker_config_data = tokio::fs::read(&invoker_config_file_path)
        .await
        .with_context(|| {
            format!(
                "unable to read config from {}",
                invoker_config_file_path.display()
            )
        })?;

    serde_yaml::from_slice(&invoker_config_data).context("config parse error")
}

/// Fetches config from Kubernetes ConfigMap.
/// Returns Ok(Some(config)) on success, Err(err) on error
/// and Ok(None) if not running inside kubernetes
pub async fn get_config_from_k8s() -> anyhow::Result<Option<invoker::config::InvokerConfig>> {
    #[cfg(feature = "k8s")]
    return get_config_from_k8s_inner().await;
    #[cfg(not(feature = "k8s"))]
    return Ok(None);
}

#[cfg(feature = "k8s")]
async fn get_config_from_k8s_inner() -> anyhow::Result<Option<invoker::config::InvokerConfig>> {
    let incluster_config = match kube::Config::from_cluster_env() {
        Ok(conf) => conf,
        Err(err) => {
            let is_caused_by_non_k8s_environment = matches!(
                &err,
                kube::error::Error::Kubeconfig(
                    kube::error::ConfigError::MissingInClusterVariables { .. },
                )
            );
            if is_caused_by_non_k8s_environment {
                return Ok(None);
            } else {
                anyhow::bail!("failed to infer k8s config: {}", err);
            }
        }
    };
    let namespace = incluster_config.default_ns.clone();
    log::info!(
        "Discovered Kuberentes API-server: url={} ns={}",
        &incluster_config.cluster_url,
        &incluster_config.default_ns
    );
    let client = kube::Client::new(incluster_config);

    let configmaps_api =
        kube::Api::<k8s_openapi::api::core::v1::ConfigMap>::namespaced(client, &namespace);
    let config_map_name = std::env::var("CONFIGMAP").unwrap_or_else(|_| "jjs-config".to_string());

    let configmap = configmaps_api
        .get(&config_map_name)
        .await
        .context("can not read ConfigMap with configuration")?;

    let config_map_key_name =
        std::env::var("CONFIGMAP_KEY").unwrap_or_else(|_| "judge".to_string());
    let config_data = match &configmap.data {
        Some(data) => data.get(&config_map_key_name),
        None => None,
    }
    .context("ConfigMap does not have key with configuration")?;
    serde_yaml::from_str(&config_data).context("config parse error")
}
