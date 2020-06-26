use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct InvokerConfig {
    /// How many workers should be spawned
    /// By default equal to processor count
    #[serde(default)]
    pub workers: Option<usize>,
    /// API service config
    #[serde(default)]
    pub api: ApiSvcConfig,
    /// If enabled, invoker will directly mount host filesystem instead of
    /// toolchain image.
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
    /// Configures how invoker should resolve problems
    pub problems: problem_loader::LoaderConfig,
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
