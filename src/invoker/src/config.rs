use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct InvokerConfig {
    /// How many workers should be spawned
    /// By default equal to processor count
    #[serde(default)]
    pub workers: Option<usize>,
    /// Configures how much invoker will sleep between ticks
    #[serde(default)]
    pub sleep: SleepConfig,
    /// API service config
    #[serde(default)]
    pub api: ApiSvcConfig,
    /// If enabled, invoker will directly mount host filesystem instead of
    /// JJS_DATA/opt
    #[serde(default)]
    pub host_toolchains: bool,
    /// Override directories that will be mounted into sandbox.
    /// E.g. if `expose-host-dirs = ["lib64", "usr/lib"]`,
    /// then invoker will mount:
    /// - `$SANDBOX_ROOT/lib64` -> `/lib64`
    /// - `$SANDBOX_ROOT/usr/lib` -> `/usr/lib`
    /// As usual, all mounts will be no-suid and read-only.
    #[serde(default)]
    pub expose_host_dirs: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct SleepConfig {
    /// Max sleep duration
    #[serde(default = "SleepConfig::default_max")]
    pub max_ms: u32,
    /// Growth of sleep duration if tick had not any updates
    #[serde(default = "SleepConfig::default_step")]
    pub step_ms: u32,
}

impl SleepConfig {
    fn default_max() -> u32 {
        2000
    }

    fn default_step() -> u32 {
        500
    }
}

impl Default for SleepConfig {
    fn default() -> Self {
        SleepConfig {
            max_ms: Self::default_max(),
            step_ms: Self::default_step(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
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
