use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct JudgeConfig {
    /// How many workers should be spawned
    /// By default equal to processor count
    #[serde(default)]
    pub managed_invokers: Option<usize>,
    /// Path to invoker binary.
    /// By default deduced from path to judge.
    /// If `managed_invokers` set to 0, value does not matter
    #[serde(default = "JudgeConfig::default_invoker_path")]
    pub invoker_path: PathBuf,
    /// API service config
    #[serde(default)]
    pub api: ApiSvcConfig,
    /// Configures how invoker should resolve problems
    pub problems: problem_loader::LoaderConfig,
}

impl JudgeConfig {
    fn default_invoker_path() -> PathBuf {
        let self_path = std::env::current_exe().expect("failed to get path to self");
        let parent = self_path
            .parent()
            .expect("path to file must contain at least one component");
        parent.join("jjs-invoker")
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ApiSvcConfig {
    /// Override bind IP
    #[serde(default = "ApiSvcConfig::default_address")]
    pub address: String,
    /// Override bind port
    #[serde(default = "ApiSvcConfig::default_port")]
    pub port: u16,
}

impl ApiSvcConfig {
    fn default_address() -> String {
        "0.0.0.0".to_string()
    }

    fn default_port() -> u16 {
        1789
    }
}

impl Default for ApiSvcConfig {
    fn default() -> Self {
        ApiSvcConfig {
            address: ApiSvcConfig::default_address(),
            port: ApiSvcConfig::default_port(),
        }
    }
}
