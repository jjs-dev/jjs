use crate::{artifact::Artifact, cfg::BuildProfile, package::RustPackage, Params};
use anyhow::Context as _;
use std::process::Command;
use util::cmd::CommandExt as _;

/// Builder takes several RustPackage-s and turns them into artifacts
pub struct Builder<'a> {
    params: &'a Params,
    packages: Vec<RustPackage>,
}

impl<'a> Builder<'a> {
    pub(crate) fn new(params: &'a Params) -> Self {
        Builder {
            params,
            packages: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, pkg: RustPackage) {
        self.packages.push(pkg);
    }

    pub(crate) fn build(self) -> anyhow::Result<Vec<Artifact>> {
        let mut cmd = Command::new("cargo");
        cmd.env("CARGO_PROFILE_RELEASE_INCREMENTAL", "false");
        cmd.current_dir(&self.params.src);
        cmd.arg("build");
        if let Some(target) = &self.params.cfg.build.target {
            cmd.args(&["--target", target]);
        }
        let profile = self.params.cfg.build.profile;
        if let BuildProfile::Release | BuildProfile::RelWithDebInfo = profile {
            cmd.arg("--release");
        }
        if let BuildProfile::RelWithDebInfo = profile {
            cmd.env("CARGO_PROFILE_RELEASE_DEBUG", "true");
        }
        cmd.env("CARGO_TARGET_DIR", &self.params.build);
        cmd.arg("-Zunstable-options");
        cmd.arg("--out-dir").arg(self.params.build.join("jjs-out"));
        cmd.arg("-Zpackage-features");
        for feat in &self.params.cfg.build.features {
           cmd.arg("--features").arg(feat);
        }
        cmd.arg("--locked");
        for pkg in &self.packages {
            cmd.arg("--package").arg(&pkg.name);
        }
        cmd.try_exec().context("can not compile")?;
        let artifacts = self
            .packages
            .iter()
            .map(|pkg| Artifact {
                package_name: pkg.name.clone(),
            })
            .collect();
        Ok(artifacts)
    }
}
