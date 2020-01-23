//! Abstracts installing to directory, deb, etc
use crate::Params;
use std::{
    path::{Path, PathBuf},
    process::exit,
};

pub struct InstallCtx<'ictx> {
    /// Sysroot-like dir
    params: &'ictx Params,
}

impl<'ictx> InstallCtx<'ictx> {
    pub(crate) fn new(params: &'ictx Params) -> Self {
        Self { params }
    }

    fn artifacts(&self) -> &Path {
        &self.params.artifacts
    }

    fn non_bin_out_dir(&self) -> &Path {
        &self.params.build
    }

    fn bin_out_dir(&self) -> PathBuf {
        self.params.build.join("jjs-out")
    }

    fn copy(&self, src: impl AsRef<Path>, dest: impl AsRef<Path>) {
        let src = src.as_ref().to_path_buf();
        let dest = dest.as_ref().to_path_buf();
        if let Err(err) = std::fs::copy(&src, &dest) {
            eprintln!(
                "Error when copying {} to {}: {}",
                src.display(),
                dest.display(),
                err
            );
            exit(1);
        }
    }

    pub(crate) fn add_bin_pkg(&self, name: &str, inst_name: &str) {
        let dest = self.artifacts().join("bin").join(inst_name);
        crate::util::ensure_exists(&dest.parent().unwrap()).unwrap();
        self.copy(self.bin_out_dir().join(name), &dest);
    }

    fn preprocess_dylib_name(name: &str) -> String {
        format!("lib{}.so", name).replace('-', "_")
    }

    fn preprocess_header_name(name: &str) -> String {
        let mut s = name.to_string();
        s.push_str(".h");
        s
    }

    pub(crate) fn add_dylib_pkg(&self, name: &str, inst_name: &str) {
        let dest = self
            .artifacts()
            .join("lib")
            .join(Self::preprocess_dylib_name(inst_name));
        crate::util::ensure_exists(&dest.parent().unwrap()).unwrap();
        self.copy(
            self.bin_out_dir().join(Self::preprocess_dylib_name(name)),
            &dest,
        );
    }

    pub(crate) fn add_header(&self, name: &str, inst_name: &str) {
        let dest = self
            .artifacts()
            .join("include/jjs")
            .join(Self::preprocess_header_name(inst_name));
        crate::util::ensure_exists(&dest.parent().unwrap()).unwrap();
        self.copy(
            self.non_bin_out_dir()
                .join(InstallCtx::preprocess_header_name(name)),
            &dest,
        );
    }
}
