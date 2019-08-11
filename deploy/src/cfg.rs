use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum BuildProfile {
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "release-dbg")]
    RelWithDebInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub cargo: String,
    pub cmake: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub profile: BuildProfile,
    pub target: String,
    pub tool_info: ToolInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentsConfig {
    pub man: bool,
    pub testlib: bool,
    pub archive: bool,
    pub tools: bool,
    pub core: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub prefix: Option<String>,
    pub verbose: bool,
    pub deb: bool,
    pub build: BuildConfig,
    pub components: ComponentsConfig,
}
