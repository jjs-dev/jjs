use crate::{
    artifact::{Artifact, CmakeArtifact, RustArtifact},
    cfg::BuildProfile,
    package::{CmakePackage, RustPackage},
    Params,
};
use anyhow::Context as _;
use std::process::Command;
use util::cmd::CommandExt as _;

/// Builder takes several RustPackage-s and turns them into artifacts
pub struct Builder<'a> {
    params: &'a Params,
    rust_packages: Vec<RustPackage>,
    cmake_packages: Vec<CmakePackage>,
}

impl<'a> Builder<'a> {
    pub(crate) fn new(params: &'a Params) -> Self {
        Builder {
            params,
            rust_packages: Vec::new(),
            cmake_packages: Vec::new(),
        }
    }

    pub(crate) fn push_rust(&mut self, pkg: RustPackage) {
        self.rust_packages.push(pkg);
    }

    pub(crate) fn push_cmake(&mut self, pkg: CmakePackage) {
        self.cmake_packages.push(pkg);
    }

    fn build_rust(&self) -> anyhow::Result<Vec<Artifact>> {
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
        for pkg in &self.rust_packages {
            cmd.arg("--package").arg(&pkg.name);
        }
        cmd.try_exec().context("can not compile")?;
        let artifacts = self
            .rust_packages
            .iter()
            .map(|pkg| {
                Artifact::Rust(RustArtifact {
                    package_name: pkg.name.clone(),
                })
            })
            .collect();
        Ok(artifacts)
    }

    fn build_cmake(&self) -> anyhow::Result<Vec<Artifact>> {
        let mut artifacts = Vec::new();
        for pkg in &self.cmake_packages {
            let install_dir = self
                .params
                .build
                .canonicalize()?
                .join("jjs-out")
                .join(&pkg.name);
            let build_dir = self.params.build.join("cmake-builds").join(&pkg.name);
            std::fs::create_dir_all(&build_dir).ok();

            let mut cmd_configure = Command::new("cmake");
            cmd_configure.arg(format!(
                "-DCMAKE_BUILD_TYPE={}",
                self.params.cfg.build.profile.as_str()
            ));
            cmd_configure.arg(format!("-DCMAKE_INSTALL_PREFIX={}", install_dir.display()));
            cmd_configure.arg(self.params.src.join("src").join(&pkg.name));
            cmd_configure.current_dir(&build_dir);
            cmd_configure.try_exec().context("failed to configure")?;

            let mut cmd_build = Command::new("cmake");
            cmd_build.current_dir(&build_dir);
            cmd_build.args(&["--build", ".", "--target", "install"]);
            cmd_build.try_exec().context("build error")?;

            artifacts.push(Artifact::Cmake(CmakeArtifact {
                package_name: pkg.name.clone(),
            }));
        }
        Ok(artifacts)
    }

    pub(crate) fn build(self) -> anyhow::Result<Vec<Artifact>> {
        let mut rust_artifacts = self.build_rust()?;
        let mut cmake_packages = self.build_cmake()?;
        rust_artifacts.append(&mut cmake_packages);
        Ok(rust_artifacts)
    }
}
