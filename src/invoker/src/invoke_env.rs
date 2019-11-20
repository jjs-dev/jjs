use std::path::PathBuf;

/// InvokeEnv is struct with various bits of data required for judging
#[derive(Clone)]
pub(crate) struct InvokeEnv<'a> {
    /// Minion backend to use for spawning children
    pub(crate) minion_backend: &'a dyn minion::Backend,
    /// JJS cluster config.
    pub(crate) cfg: &'a cfg::Config,
    /// Toolchain config.
    pub(crate) toolchain_cfg: &'a cfg::Toolchain,
    /// Problem config (from DATA_DIR/var/contests/...).
    pub(crate) problem_cfg: &'a cfg::Problem,
    /// Problem manifest (from DATA_DIR/var/problems/...)
    pub(crate) problem_data: &'a pom::Problem,
    /// Information about run
    pub(crate) run_props: &'a crate::RunProps,
}

impl<'a> InvokeEnv<'a> {
    pub(crate) fn problem_root(&self) -> PathBuf {
        self.cfg
            .sysroot
            .join("var/problems")
            .join(&self.problem_cfg.name)
    }
}
