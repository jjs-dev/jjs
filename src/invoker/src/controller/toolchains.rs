//! This module is responsible for toolchain loading
use anyhow::Context as _;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Toolchain {
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

pub struct ResolvedToolchain {
    pub configuration: Toolchain,
    pub path: PathBuf,
}

/// Responsible for fetching toolchains
pub struct ToolchainLoader {
    puller: puller::Puller,
    toolchains_dir: tempfile::TempDir,
}

impl ToolchainLoader {
    pub async fn new() -> anyhow::Result<ToolchainLoader> {
        let puller = puller::Puller::new().await;
        let toolchains_dir = tempfile::TempDir::new()?;
        Ok(ToolchainLoader {
            toolchains_dir,
            puller,
        })
    }

    /// Actually downloads and unpacks toolchain to specified dir.
    async fn extract_toolchain(
        &self,
        toolchain_url: &str,
        target_dir: &Path,
    ) -> anyhow::Result<()> {
        tokio::fs::create_dir(target_dir)
            .await
            .context("failed to create target dir")?;

        self.puller
            .pull(
                toolchain_url,
                target_dir,
                tokio::sync::CancellationToken::new(),
            )
            .await
            .context("failed to pull toolchain")?;
        Ok(())
    }

    pub async fn resolve(&self, toolchain_url: &str) -> anyhow::Result<ResolvedToolchain> {
        let toolchain_dir = self
            .toolchains_dir
            .path()
            .join(base64::encode(toolchain_url));
        if !toolchain_dir.exists() {
            self.extract_toolchain(toolchain_url, &toolchain_dir)
                .await
                .context("toolchain download error")?;
        }
        let toolchain_config_path = toolchain_dir.join("toolchain.yaml");
        let toolchain_config = tokio::fs::read(toolchain_config_path)
            .await
            .context("toolchain config file (toolchain.yaml in image root) missing")?;
        let toolchain_config: Toolchain =
            serde_yaml::from_slice(&toolchain_config).context("invalid config")?;
        Ok(ResolvedToolchain {
            path: toolchain_dir,
            configuration: toolchain_config,
        })
    }
}
