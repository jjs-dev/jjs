use crate::{cfg::BuildProfile, Params};
use std::process::Command;

pub struct BuildCtx<'bctx> {
    params: &'bctx Params,
}

impl<'bctx> BuildCtx<'bctx> {
    pub(crate) fn new(params: &'bctx Params) -> Self {
        Self { params }
    }

    pub(crate) fn cargo(&self) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.args(&["-Z", "config-profile"]);
        cmd.env("CARGO_PROFILE_RELEASE_INCREMENTAL", "false");
        cmd.current_dir(&self.params.src);
        cmd
    }

    pub(crate) fn cargo_build(&self, pkg_name: &str) -> Command {
        let mut cmd = self.cargo();
        cmd.arg("build");
        cmd.args(&["--package", pkg_name]);
        cmd.arg("--no-default-features");
        cmd.args(&["--target", &self.params.cfg.build.target]);
        let profile = self.params.cfg.build.profile;
        if let BuildProfile::Release | BuildProfile::RelWithDebInfo = profile {
            cmd.arg("--release");
        }
        if let BuildProfile::RelWithDebInfo = profile {
            cmd.env("CARGO_PROFILE_RELEASE_DEBUG", "true");
        }
        cmd
    }
}
