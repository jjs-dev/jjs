use anyhow::Context;
use invoker::controller::JudgeRequestAndCallbacks;
use std::sync::Arc;
use tracing::{info, instrument, warn};
fn is_cli_mode() -> bool {
    std::env::args().count() > 1
}

async fn start_request_providers(
    cancel: tokio::sync::CancellationToken,
    chan: async_mpmc::Sender<JudgeRequestAndCallbacks>,
) -> anyhow::Result<()> {
    if is_cli_mode() {
        info!("spawning CliSource");
        tokio::task::spawn(invoker::sources::cli_source::run(chan, cancel));
    } else {
        info!("Establishing apiserver connection");
        let api = client::infer().await.context("API connection failed")?;
        info!("Spawning ApiSource");
        let api_source = invoker::sources::ApiSource::new(api, chan);
        tokio::task::spawn(async move {
            api_source.run(cancel).await;
        });
    }
    Ok(())
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
    }
    let mut rt = tokio::runtime::Builder::new();
    rt.basic_scheduler();
    if is_worker() {
        rt.max_threads(4);
    }
    let mut rt = rt.enable_all().core_threads(1).build()?;
    rt.block_on(async {
        let cancel_token = tokio::sync::CancellationToken::new();
        let res = real_main(cancel_token.clone()).await;
        if res.is_err() {
            cancel_token.cancel();
        }
        res
    })
}
#[instrument(skip(judge_requests))]
async fn start_controller(
    config: Arc<invoker::config::InvokerConfig>,
    system_config_data: util::cfg::CfgData,
    judge_requests: async_mpmc::Receiver<JudgeRequestAndCallbacks>,
) -> anyhow::Result<()> {
    info!("Starting controller");
    let controller = invoker::controller::Controller::new(system_config_data, config)
        .await
        .context("failed to start controller")?;
    controller.exec_on(judge_requests);
    Ok(())
}

async fn real_main(cancel_token: tokio::sync::CancellationToken) -> anyhow::Result<()> {
    if is_worker() {
        return invoker::worker::main().await;
    }

    let system_config_data = util::cfg::load_cfg_data()?;

    // now we should fetch InvokerConfig
    // we have generic `get_config_from_fs` and specific `get_config_from_k8s`
    let invoker_config = {
        if let Some(cfg) = get_config_from_k8s().await? {
            info!("Got config from Kubernetes");
            cfg
        } else {
            info!("Loading config from FS");
            get_config_from_fs(&system_config_data).await?
        }
    };
    // TODO probably broken for IPv6
    let bind_address = format!("{}:{}", invoker_config.api.address, invoker_config.api.port);
    let bind_address = bind_address
        .parse()
        .with_context(|| format!("invalid bind address {}", bind_address))?;

    let (judge_request_tx, judge_request_rx) = async_mpmc::channel();

    invoker::api::start(cancel_token.clone(), bind_address, judge_request_tx.clone())
        .await
        .context("failed to start api")?;

    info!("API service started");
    start_request_providers(cancel_token.clone(), judge_request_tx)
        .await
        .context("failed to initialize request providers")?;
    start_controller(
        Arc::new(invoker_config),
        system_config_data,
        judge_request_rx,
    )
    .await
    .context("can not start controller")?;
    {
        let cancel_token = cancel_token.clone();
        tokio::task::spawn(async move {
            info!("Installing signal hook");
            tokio::select! {
                res = tokio::signal::ctrl_c() => {
                    match res {
                        Ok(_) => {
                            info!("Received ctrl-c");
                            cancel_token.cancel();
                        }
                        Err(err) => warn!(error=%err, "Failed to wait for signal"),
                    }
                }
                _ = cancel_token.cancelled() => ()
            }
        });
    }
    cancel_token.cancelled().await;
    info!("Received shutdown request; exiting gracefully");
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

    info!(path=%invoker_config_file_path.display(), "Found config");

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
    info!(cluster_api_url=%incluster_config.cluster_url,
        namespace=%incluster_config.default_ns,
        "Discovered Kuberentes API-server",
    );
    let client = kube::Client::new(incluster_config);

    let configmaps_api =
        kube::Api::<k8s_openapi::api::core::v1::ConfigMap>::namespaced(client, &namespace);
    let config_map_name = std::env::var("CONFIGMAP").unwrap_or_else(|_| "jjs-config".to_string());

    let configmap = configmaps_api
        .get(&config_map_name)
        .await
        .context("can not read ConfigMap with configuration")?;

    tracing::debug!("Resolved ConfigMap");

    let config_map_key_name =
        std::env::var("CONFIGMAP_KEY").unwrap_or_else(|_| "judge".to_string());
    let config_data = match &configmap.data {
        Some(data) => data.get(&config_map_key_name),
        None => None,
    }
    .context("ConfigMap does not have key with configuration")?;
    serde_yaml::from_str(&config_data).context("config parse error")
}
