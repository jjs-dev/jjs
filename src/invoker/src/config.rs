use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
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
    /// Directory which will contain temporary invocation data.
    pub work_root: PathBuf,
}
