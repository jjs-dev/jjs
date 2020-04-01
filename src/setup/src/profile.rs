use serde::Deserialize;
use std::path::PathBuf;
/// Profile contains all settings and other data, representing desired state
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Profile {
    pub(crate) data_dir: Option<PathBuf>,
    pub(crate) install_dir: PathBuf,
    pub(crate) pg: Option<PgProfile>,
    pub(crate) toolchains: Option<TcsProfile>,
    pub(crate) problems: Option<ProblemsProfile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PgProfile {
    pub(crate) conn_string: String,
    pub(crate) db_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct TcsProfile {
    /// All toolchains from this list will be skipped
    #[serde(default)]
    pub(crate) blacklist: Vec<String>,
    /// If non-empty, all toolchains not from this list will be skipped
    #[serde(default)]
    pub(crate) whitelist: Vec<String>,
    /// Strategies used by `jjs-configure-toolchains`. If empty, default list will be used.
    #[serde(default)]
    pub(crate) strategies: Vec<String>,
    /// Will be appended to `jjs-configure-toolchains` argv.
    #[serde(default)]
    pub(crate) additional_args: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ProblemsProfile {
    pub(crate) tasks: Vec<ProblemTask>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ProblemTask {
    pub(crate) source: ProblemSource,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub(crate) enum ProblemSource {
    Path { path: std::path::PathBuf },
}
