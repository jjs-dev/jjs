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

struct ImageManager {
    /// Which container manager to use - docker/podman
    manager: String,
}

async fn binary_exists(binary_name: &str) -> anyhow::Result<bool> {
    let mut cmd = tokio::process::Command::new("which");
    cmd.arg(binary_name);
    let status = cmd.status().await.context("spawn error")?;
    Ok(status.success())
}

impl ImageManager {
    async fn new() -> anyhow::Result<ImageManager> {
        if let Ok(hint) = std::env::var("JJS_INVOKER_IMAGE_MANAGER") {
            return Ok(ImageManager { manager: hint });
        }
        if binary_exists("podman").await? {
            return Ok(ImageManager {
                manager: "podman".to_string(),
            });
        }
        if binary_exists("docker").await? {
            return Ok(ImageManager {
                manager: "docker".to_string(),
            });
        }
        anyhow::bail!("can not find image manager")
    }

    async fn save(&self, image_name: &str, target_dir: &Path) -> anyhow::Result<()> {
        let container_id = {
            let mut spawn_cmd = tokio::process::Command::new(&self.manager);
            spawn_cmd.arg("create");
            spawn_cmd.arg(image_name);
            let spawn_out = spawn_cmd.output().await.context("process spawn error")?;
            if !spawn_out.status.success() {
                eprintln!("stdout:\n{}", String::from_utf8_lossy(&spawn_out.stdout));
                eprintln!("stderr:\n{}", String::from_utf8_lossy(&spawn_out.stderr));
                anyhow::bail!("container create failed: exit code {:?}", spawn_out.status);
            }
            let container_id =
                String::from_utf8(spawn_out.stdout).context("corrupted create output")?;

            container_id.trim().to_string()
        };
        {
            let mut copy_cmd = tokio::process::Command::new(&self.manager);
            copy_cmd.arg("copy");
            // copy all filesystem of container...
            copy_cmd.arg(format!("{}:/", &container_id));
            // ...to `target_dir`
            copy_cmd.arg(target_dir);
            let copy_out = copy_cmd.output().await.context("process spawn error")?;
            if !copy_out.status.success() {
                eprintln!("stdout:\n{}", String::from_utf8_lossy(&copy_out.stdout));
                eprintln!("stderr:\n{}", String::from_utf8_lossy(&copy_out.stderr));
                anyhow::bail!("copy failed: exit code {:?}", copy_out.status);
            }
        }
        {
            let mut kill_cmd = tokio::process::Command::new(&self.manager);
            kill_cmd.arg("rm");
            kill_cmd.arg(container_id);
            let kill_out = kill_cmd.output().await.context("process spawn error")?;
            if !kill_out.status.success() {
                eprintln!("stdout:\n{}", String::from_utf8_lossy(&kill_out.stdout));
                eprintln!("stderr:\n{}", String::from_utf8_lossy(&kill_out.stderr));
                anyhow::bail!("kill failed: exit code {:?}", kill_out.status);
            }
        }

        Ok(())
    }
}

/// Responsible for fetching toolchains
pub struct ToolchainLoader {
    image_manager: ImageManager,
    toolchains_dir: tempfile::TempDir,
}

impl ToolchainLoader {
    pub async fn new() -> anyhow::Result<ToolchainLoader> {
        let image_manager = ImageManager::new().await?;
        let toolchains_dir = tempfile::TempDir::new()?;
        Ok(ToolchainLoader {
            image_manager,
            toolchains_dir,
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
        self.image_manager
            .save(toolchain_url, target_dir)
            .await
            .context("failed to save toolchain")?;
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
