use crate::{
    build_ctx::BuildCtx,
    inst_ctx::InstallCtx,
    pkg::{Package, PackageComponentKind},
    sel_ctx::SelCtx,
    util::print_section,
};
use std::path::PathBuf;
use util::cmd::CommandExt;

#[derive(Debug)]
pub(crate) struct BinPackage {
    package_name: String,
    install_name: String,
    component_kind: PackageComponentKind,
    selected: Option<bool>,
    path: Option<PathBuf>,
}

impl BinPackage {
    pub(crate) fn new(pkg_name: &str, inst_name: &str, comp: PackageComponentKind) -> Self {
        Self {
            package_name: pkg_name.to_string(),
            install_name: inst_name.to_string(),
            component_kind: comp,
            selected: None,
            path: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct BinPackages {
    pkgs: Vec<BinPackage>,
    selected: Vec<bool>,
}

impl BinPackages {
    pub(crate) fn new(pkgs: Vec<BinPackage>) -> Self {
        let n = pkgs.len();
        BinPackages {
            pkgs,
            selected: vec![false; n],
        }
    }
}

impl Package for BinPackages {
    fn check_selected(&mut self, sctx: &SelCtx) {
        for i in 0..self.pkgs.len() {
            let res = match self.pkgs[i].component_kind {
                PackageComponentKind::Core => sctx.components_cfg().core,
                PackageComponentKind::Extra => sctx.components_cfg().extras,
                PackageComponentKind::Tools => sctx.components_cfg().tools,
            };
            self.selected[i] = res;
        }
    }

    fn selected(&self) -> bool {
        true
    }

    fn build(&self, bctx: &BuildCtx) {
        let mut section_title = "Building".to_string();
        let mut is_first_pkg = true;
        let mut cmd = bctx.cargo_build();
        for i in 0..self.pkgs.len() {
            if !self.selected[i] {
                continue;
            }
            if is_first_pkg {
                section_title += " ";
                is_first_pkg = false;
            } else {
                section_title += ", ";
            }
            section_title += &self.pkgs[i].package_name;
            cmd.arg("--package").arg(&self.pkgs[i].package_name);
        }
        print_section(&section_title);
        cmd.run_on(bctx.runner());
    }

    fn install(&self, ictx: &InstallCtx) {
        for i in 0..self.pkgs.len() {
            if !self.selected[i] {
                continue;
            }
            ictx.add_bin_pkg(&self.pkgs[i].package_name, &self.pkgs[i].install_name);
        }
    }
}

#[derive(Debug)]
pub(crate) struct MinionFfiPackage {
    selected: Option<bool>,
    path: Option<PathBuf>,
}

impl Package for MinionFfiPackage {
    fn check_selected(&mut self, sctx: &SelCtx) {
        self.selected = Some(sctx.components_cfg().extras);
    }

    fn selected(&self) -> bool {
        self.selected.unwrap()
    }

    fn build(&self, bctx: &BuildCtx) {
        let st = bctx
            .cargo_build()
            .args(&["-p", "minion-ffi"])
            .status()
            .unwrap()
            .success();
        assert_eq!(st, true);
    }

    fn install(&self, inst_mgr: &InstallCtx) {
        inst_mgr.add_dylib_pkg("minion-ffi", "jjs_minion_ffi");
        inst_mgr.add_header("minion-ffi", "minion-ffi");
        inst_mgr.add_header("minion-ffi-prepend", "minion-ffi-prepend");
    }
}

impl MinionFfiPackage {
    pub(crate) fn new() -> Self {
        Self {
            selected: None,
            path: None,
        }
    }
}
