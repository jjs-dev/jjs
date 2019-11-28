use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SetupKind {
    Trace,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ToolchainConfig {
    kind: SetupKind,
    #[serde(default)]
    pub(crate) depends: Vec<String>,
    #[serde(default)]
    pub(crate) auto: bool,
}
