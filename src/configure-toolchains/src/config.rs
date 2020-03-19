use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Strategy {
    Trace,
    Debootstrap,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ToolchainConfig {
    pub(crate) strategies: Vec<Strategy>,
    #[serde(default)]
    pub(crate) depends: Vec<String>,
    #[serde(default)]
    pub(crate) auto: bool,
}
