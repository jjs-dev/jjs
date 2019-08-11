//! Abstractions for package
use crate::{
    cfg::{BuildConfig, ComponentsConfig, BuildProfile},
    util::print_section,
};
use std::{path::PathBuf, process::Command};

#[derive(Debug, Clone)]
pub(crate) enum PackageKind {
    Bin,
    Dylib,
}

// TODO: refactor
pub(crate) struct BuildProps {
    pub(crate) build_dir: PathBuf,
    pub(crate) project_dir: PathBuf,
}

pub(crate) trait Package: std::fmt::Debug {
    fn kind(&self) -> PackageKind;

    fn install_name(&self) -> String;

    fn selected(&self, cfg: &ComponentsConfig) -> bool;

    fn build(&self, cfg: &BuildConfig, props: &BuildProps);
}

pub(crate) enum PackageComponentKind {
    Core,
    Tools,
    Custom(Box<dyn Fn(&ComponentsConfig) -> bool>),
}

mod pkg_comp_kind_debug_impl {
    use super::*;
    use std::fmt::*;

    impl Debug for PackageComponentKind {
        fn fmt(&self, f: &mut Formatter) -> Result {
            match self {
                PackageComponentKind::Core => {
                    f.write_str("Core")
                }
                PackageComponentKind::Tools => {
                    f.write_str("Tools")
                }
                PackageComponentKind::Custom(_) => {
                    f.write_str("Custom")
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct BinPackage {
    pub(crate) package_name: String,
    pub(crate) install_name: String,
    pub(crate) component_kind: PackageComponentKind,
    pub(crate) features: Vec<String>,
}


impl Package for BinPackage {
    fn kind(&self) -> PackageKind {
        PackageKind::Bin
    }

    fn install_name(&self) -> String {
        self.install_name.clone()
    }

    fn selected(&self, comps_cfg: &ComponentsConfig) -> bool {
        match &self.component_kind {
            PackageComponentKind::Core => comps_cfg.core,
            PackageComponentKind::Tools => comps_cfg.tools,
            PackageComponentKind::Custom(f) => f(comps_cfg),
        }
    }

    fn build(&self, build_cfg: &BuildConfig, props: &BuildProps) {
        print_section(&format!("Building {}", &self.package_name));
        let mut cmd = Command::new(&build_cfg.tool_info.cargo);

        cmd.args(&["-Z", "config-profile"]);
        cmd.current_dir(&props.project_dir).args(&[
            "build",
            "--package",
            &self.package_name,
            "--target",
            &build_cfg.target,
        ]);
        cmd.arg("--no-default-features");
        if !self.features.is_empty() {
            cmd.arg("--features");
            let feat = self.features.join(",");
            cmd.arg(&feat);
        }
        let profile = build_cfg.profile;
        if let BuildProfile::Release | BuildProfile::RelWithDebInfo = profile {
            cmd.arg("--release");
        }
        if let BuildProfile::RelWithDebInfo = profile {
            cmd.env("CARGO_PROFILE_RELEASE_DEBUG", "true");
        }
        cmd.env("CARGO_PROFILE_RELEASE_INCREMENTAL", "false");
        let st = cmd.status().unwrap().success();
        assert_eq!(st, true);
    }
}
