//! Abstracts installing to directory, deb, etc
use crate::{cfg::BuildProfile, Params};
use std::path::{Path, PathBuf};

pub struct InstallCtx<'ictx> {
    /// Sysroot-like dir
    params: &'ictx Params,
}

impl<'ictx> InstallCtx<'ictx> {
    pub(crate) fn new(params: &'ictx Params) -> Self {
        Self { params }
    }

    fn artifacts(&self) -> &Path {
        &self.params.sysroot
    }

    fn non_arch_out_dir(&self) -> PathBuf {
        self.params.build.clone()
    }

    /// Returns {TARGET_DIR}/{TARGET_ARCH}/{BUILD_PROFILE}
    fn out_dir(&self) -> PathBuf {
        let mut p = self.non_arch_out_dir();
        p.push("x86_64-unknown-linux-gnu");
        match self.params.cfg.build.profile {
            BuildProfile::Debug => {
                p.push("debug");
            }
            BuildProfile::Release | BuildProfile::RelWithDebInfo => {
                p.push("release");
            }
        };
        p
    }

    fn artifact_path(&self, name: &str) -> PathBuf {
        let mut p = self.out_dir();
        p.push(name);
        p
    }

    pub(crate) fn add_bin_pkg(&self, name: &str, inst_name: &str) {
        let dest = self.artifacts().join("bin").join(inst_name);
        crate::util::ensure_exists(&dest.parent().unwrap()).unwrap();
        std::fs::copy(self.artifact_path(name), &dest).unwrap();
    }

    pub(crate) fn add_dylib_pkg(&self, name: &str, inst_name: &str) {
        let dest = self.artifacts().join("lib").join(inst_name);
        crate::util::ensure_exists(&dest.parent().unwrap()).unwrap();
        std::fs::copy(self.artifact_path(name), &dest).unwrap();
    }

    pub(crate) fn add_header(&self, name: &str, inst_name: &str) {
        let dest = self.artifacts().join("include/jjs").join(inst_name);
        crate::util::ensure_exists(&dest.parent().unwrap()).unwrap();
        std::fs::copy(self.non_arch_out_dir().join(name), &dest).unwrap();
    }
}
