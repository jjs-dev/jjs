use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SetupKind {
    Trace,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    kind: SetupKind,
}

impl Config {}
