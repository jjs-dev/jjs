use crate::{
    pkg::{
        Package, BinPackage,
        PackageComponentKind}
};
use crate::inst_mgr::InstallManager;
use crate::pkg::BuildProps;

pub struct BuildManager {
    packages: Vec<Box<dyn Package>>
}

impl BuildManager {
    pub(crate) fn new() -> Self {
        Self {
            packages: vec![]
        }
    }

    pub(crate) fn add(&mut self, pkg: Box<dyn Package>) -> &mut Self {
        self.packages.push(pkg);
        self
    }

    pub(crate) fn add_bin(&mut self, pkg_name: &str, inst_name: &str, comp: PackageComponentKind) -> &mut Self {
        let pkg = BinPackage {
            package_name: pkg_name.to_string(),
            install_name: inst_name.to_string(),
            component_kind: comp,
            features: vec![],
        };
        self.packages.push(Box::new(pkg));
        self
    }

    pub(crate) fn build(self, params: &crate::Params, inst_mgr: &mut InstallManager) {
        let build_props = BuildProps {
            build_dir: params.build.into(),
            project_dir: params.src.into(),
        }
        for pkg in self.packages {
            if pkg.selected(&config.components) {
                pkg.build(&config.build)
            }
        }
    }
}