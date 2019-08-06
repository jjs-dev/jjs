use serde::{Deserialize, Serialize};
//use strum_macros::{EnumString, Display};
use std::fmt::{self, Display, Formatter};

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
pub struct Config {
    pub prefix: Option<String>,
    pub target: String,
    pub profile: BuildProfile,
    pub man: bool,
    pub testlib: bool,
    pub archive: bool,
    pub tools: bool,
    pub tool_info: ToolInfo,
    pub verbose: bool,
    pub core: bool,
}
