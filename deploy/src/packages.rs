use crate::{
    build_ctx::BuildCtx,
    inst_ctx::InstallCtx,
    pkg::{Package, PackageComponentKind},
    sel_ctx::SelCtx,
    util::print_section,
};
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct BinPackage {
    package_name: String,
    install_name: String,
    component_kind: PackageComponentKind,
    features: Vec<String>,
    selected: Option<bool>,
    path: Option<PathBuf>,
}

impl BinPackage {
    pub(crate) fn new(pkg_name: &str, inst_name: &str, comp: PackageComponentKind) -> Self {
        Self {
            package_name: pkg_name.to_string(),
            install_name: inst_name.to_string(),
            component_kind: comp,
            features: vec![],
            selected: None,
            path: None,
        }
    }

    pub(crate) fn feature(&mut self, feat: &str) {
        self.features.push(feat.to_string());
    }
}

impl Package for BinPackage {
    fn check_selected(&mut self, sctx: &SelCtx) {
        let res = match &self.component_kind {
            PackageComponentKind::Core => sctx.components_cfg().core,
            PackageComponentKind::Extra => sctx.components_cfg().extras,
            PackageComponentKind::Tools => sctx.components_cfg().tools,
        };
        self.selected = Some(res);
    }

    fn selected(&self) -> bool {
        self.selected.unwrap()
    }

    fn build(&self, bctx: &BuildCtx) {
        print_section(&format!("Building {}", &self.package_name));
        let mut cmd = bctx.cargo_build(&self.package_name);

        if !self.features.is_empty() {
            cmd.arg("--features");
            let feat = self.features.join(",");
            cmd.arg(&feat);
        }
        let st = cmd.status().unwrap().success();
        assert_eq!(st, true);
    }

    fn install(&self, ictx: &InstallCtx) {
        ictx.add_bin_pkg(&self.package_name, &self.install_name);
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
        let st = bctx.cargo_build("minion-ffi").status().unwrap().success();
        assert_eq!(st, true);
    }

    fn install(&self, inst_mgr: &InstallCtx) {
        inst_mgr.add_dylib_pkg("minion-ffi", "jjs_minion_ffi");
        inst_mgr.add_header("minion-ffi", "minion-ffi");
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
