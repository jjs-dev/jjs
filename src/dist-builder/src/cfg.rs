use std::path::PathBuf;

#[derive(Copy, Clone, Debug)]
pub enum BuildProfile {
    Debug,
    Release,
    RelWithDebInfo,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub cargo: String,
    pub cmake: String,
    pub docker: String,
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub profile: BuildProfile,
    pub target: Option<String>,
    pub tool_info: ToolInfo,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EmitConfig {
    /// Some => docker images are built
    pub docker: Option<DockerConfig>,
}
#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub build_options: Vec<String>,
    /// None => default tag used
    pub tag: Option<String>,
}

/// Describes which components should be build
#[derive(Debug, Clone)]
pub struct ComponentsConfig {
    /// Specifies components (e.g. apiserver) to enable
    pub components: Vec<String>,
    /// Specifies sections (e.g. tool) to enable
    pub sections: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub artifacts_dir: PathBuf,
    pub verbose: bool,
    pub emit: EmitConfig,
    pub build: BuildConfig,
    pub components: ComponentsConfig,
}
