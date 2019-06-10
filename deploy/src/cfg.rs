use serde::{Deserialize, Serialize};
//use strum_macros::{EnumString, Display};
use std::fmt::{self, Display, Formatter};

// some utilities for pretty-printing

struct DebugDisplay<T>(T);

impl<T: Display> std::fmt::Debug for DebugDisplay<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

trait DebugStructExt {
    fn field_ext(&mut self, name: &str, val: &dyn Display) -> &mut Self;
}

impl<'a, 'b> DebugStructExt for fmt::DebugStruct<'a, 'b> {
    fn field_ext(&mut self, name: &str, val: &dyn Display) -> &mut Self {
        self.field(name, &DebugDisplay(val))
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum BuildProfile {
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "release-dbg")]
    RelWithDebInfo,
}

impl Display for BuildProfile {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let msg = match self {
            BuildProfile::Debug => "unoptimized with debugging symbols",
            BuildProfile::Release => "optimized",
            BuildProfile::RelWithDebInfo => "optimized with debugging symbols",
        };
        f.write_str(msg)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub cargo: String,
    pub cmake: String,
}

impl Display for ToolInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("")
            .field("CMake", &self.cmake)
            .field("Cargo", &self.cargo)
            .finish()
    }
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
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Build config")
            .field("Install prefix", &self.prefix)
            .field("Target triple", &self.target)
            .field_ext("Build profile", &self.profile)
            .field("With manual", &self.man)
            .field("With archive", &self.archive)
            .field("With testlib", &self.testlib)
            .field("With additional tools", &self.tools)
            .field_ext("External tools", &self.tool_info)
            .finish()
    }
}
