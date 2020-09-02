//! This module is responsible for toolchain loading
use anyhow::Context as _;
use dkregistry::v2::manifest::{Manifest, RuntimeConfig};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracing::{debug, instrument};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ToolchainSpec {
    /// Human-readable
    pub title: String,

    /// Machine-readable
    pub name: String,

    pub filename: String,

    #[serde(rename = "build")]
    pub build_commands: Vec<Command>,

    #[serde(rename = "run")]
    pub run_command: Command,

    #[serde(rename = "build-limits", default)]
    pub limits: pom::Limits,

    #[serde(rename = "env", default)]
    pub env: HashMap<String, String>,

    #[serde(rename = "env-passing", default)]
    pub env_passing: bool,

    #[serde(rename = "env-blacklist", default)]
    pub env_blacklist: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct Command {
    #[serde(default = "Command::default_env")]
    pub env: HashMap<String, String>,
    pub argv: Vec<String>,
    #[serde(default = "Command::default_cwd")]
    pub cwd: String,
}

impl Command {
    fn default_env() -> HashMap<String, String> {
        HashMap::new()
    }

    fn default_cwd() -> String {
        String::from("/jjs")
    }
}

#[derive(Clone)]
pub struct ResolvedToolchainInfo {
    spec: ToolchainSpec,
    pub path: PathBuf,
    image_config: ImageConfig,
}
impl ResolvedToolchainInfo {
    /// Returns toolchain spec, with applied information from docker image
    pub fn get_spec(&self) -> ToolchainSpec {
        let mut tc = self.spec.clone();
        for (k, v) in self.image_config.environment.clone() {
            tc.env.insert(k, v);
        }
        tc
    }
}

/// Contains some data, extracted from image manifest
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ImageConfig {
    pub environment: Vec<(String, String)>,
}

impl ImageConfig {
    fn parse_env_item(item: &str) -> Option<(String, String)> {
        let mut parts = item.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next()?;
        Some((key.to_string(), value.to_string()))
    }

    fn from_run_config(rc: RuntimeConfig) -> anyhow::Result<Self> {
        let environment = rc
            .env
            .unwrap_or_default()
            .into_iter()
            .map(|item| ImageConfig::parse_env_item(&item))
            .map(|item| item.context("environment string does not look like key=value"))
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self { environment })
    }
}

/// Responsible for fetching toolchains
pub struct ToolchainLoader {
    puller: puller::Puller,
    toolchains_dir: tempfile::TempDir,
    /// Cache for already pulled toolchains.
    cache: HashMap<String, ResolvedToolchainInfo>,
}

impl ToolchainLoader {
    pub async fn new() -> anyhow::Result<ToolchainLoader> {
        let puller = puller::Puller::new().await;
        let toolchains_dir = tempfile::TempDir::new()?;
        Ok(ToolchainLoader {
            toolchains_dir,
            puller,
            cache: HashMap::new(),
        })
    }

    /// Actually downloads and unpacks toolchain to specified dir.
    #[instrument(skip(self, toolchain_url, target_dir))]
    async fn extract_toolchain(
        &self,
        toolchain_url: &str,
        target_dir: &Path,
    ) -> anyhow::Result<ImageConfig> {
        debug!(target_dir=%target_dir.display(), "downloading image");
        tokio::fs::create_dir(target_dir)
            .await
            .context("failed to create target dir")?;

        let image_manifest = self
            .puller
            .pull(
                toolchain_url,
                target_dir,
                tokio::sync::CancellationToken::new(),
            )
            .await
            .context("failed to pull toolchain")?;
        let image_manifest = match image_manifest {
            Manifest::S2(im_v2) => im_v2,
            _ => anyhow::bail!("Unsupported manifest: only schema2 is supported"),
        };
        let config_blob = image_manifest.config_blob;

        let runtime_config = config_blob
            .runtime_config
            .context("image manifest does not have RunConfig")?;

        let image_config = ImageConfig::from_run_config(runtime_config)
            .context("failed to process config blob")?;
        debug!("toolchain has been pulled successfully");
        Ok(image_config)
    }

    #[instrument(skip(self))]
    pub async fn resolve(&self, toolchain_url: &str) -> anyhow::Result<ResolvedToolchainInfo> {
        if let Some(info) = self.cache.get(toolchain_url) {
            return Ok(info.clone());
        }
        let toolchain_dir = self
            .toolchains_dir
            .path()
            .join(base64::encode(toolchain_url));

        let image_config = self
            .extract_toolchain(toolchain_url, &toolchain_dir)
            .await
            .context("toolchain download error")?;

        let toolchain_spec_path = toolchain_dir.join("manifest.yaml");
        let toolchain_spec = tokio::fs::read(toolchain_spec_path)
            .await
            .context("toolchain config file (manifest.yaml in image root) missing")?;
        let toolchain_spec: ToolchainSpec =
            serde_yaml::from_slice(&toolchain_spec).context("invalid toolchain spec")?;
        Ok(ResolvedToolchainInfo {
            path: toolchain_dir,
            spec: toolchain_spec,
            image_config,
        })
    }
}
