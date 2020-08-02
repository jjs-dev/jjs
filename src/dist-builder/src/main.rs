//! This component is responsible for actually building JJS
//!
//! # Package
//! Package is unit of JJS source code. E.g. apiserver, invoker, cli, and so on.
//! Package can be
//! - Rust
//! - Other
//! - Meta
//! # Artifact
//! Artifact is result of building RustPackage. It is binary file.
//! `OtherPackage`s do not support creating Artifacts. They provide Dockefile
//! which must abstract building package image
//! # Emitter
//! Composes all Artifacts (i.e. non-Rust packages are not supported) in some
//! structured repr, e.g. Docker image or sysroot-like archive.
//! # Meta packages
//! This packages are build with special context dir, which allows them pull
//! outputs of other packages
mod artifact;
mod builder;
mod cfg;
mod emit;
mod fs_util;
mod package;

use crate::{
    cfg::BuildProfile,
    package::{CmakePackage, MetaPackage, OtherPackage, RustPackage, Section},
};
use anyhow::Context as _;
use std::path::PathBuf;
use structopt::StructOpt as _;

#[derive(structopt::StructOpt)]
struct Opt {
    /// Directory used for build files
    #[structopt(long = "build-dir", default_value = "target")]
    build_dir: PathBuf,
    /// Components and sections to enable
    ///
    /// Available sections: tools, daemons, suggested.
    /// Available components: apiserver, invoker.
    #[structopt(long = "enable")]
    enable: Vec<String>,
    /// Cargo path
    #[structopt(long, env = "CARGO")]
    cargo: Option<String>,
    /// CMake path
    #[structopt(long, env = "CMAKE")]
    cmake: Option<String>,
    /// Target triple
    #[structopt(long = "target", short = "T")]
    target: Option<String>,
    /// Optimization
    #[structopt(long = "optimize", short = "O")]
    optimize: bool,
    /// Debug symbols
    #[structopt(long = "dbg-dym", short = "D")]
    dbg_sym: bool,
    /// Emit verbose information about build
    #[structopt(long = "verbose", short = "V")]
    verbose: bool,
    /// Destination for artifacts
    #[structopt(long = "out", short = "P")]
    out_dir: PathBuf,
    /// Build docker images
    #[structopt(long = "enable-docker")]
    docker: bool,
    /// Docker image tag
    #[structopt(long)]
    docker_tag: Option<String>,
    /// Docker build additional options
    #[structopt(long)]
    docker_build_opt: Vec<String>,
    /// Name or path to Docker or other tool which can run containers (e.g. Podman)
    #[structopt(long = "with-docker")]
    docker_name: Option<String>,
    /// If set, tags of built images will be written to this file,
    /// each file on separate line
    #[structopt(long)]
    docker_tags_log: Option<PathBuf>,
    /// Features to enable
    #[structopt(long = "enable-feature")]
    features: Vec<String>,
}

fn find_docker<'a>() -> &'a str {
    let has_podman = std::process::Command::new("podman")
        .arg("--help")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_or(false, |st| st.success());
    if has_podman { "podman" } else { "docker" }
}

fn main() {
    util::log::setup();
    let jjs_src_path = std::env::current_dir().unwrap();
    let opt: Opt = Opt::from_args();

    let tool_info = cfg::ToolInfo {
        cargo: opt.cargo.as_deref().unwrap_or_else(|| "cargo").to_string(),
        cmake: opt.cmake.as_deref().unwrap_or_else(|| "cmake").to_string(),
        docker: opt
            .docker_name
            .as_deref()
            .unwrap_or_else(find_docker)
            .to_string(),
    };
    let profile = match (opt.optimize, opt.dbg_sym) {
        (true, false) => BuildProfile::Release,
        (true, true) => BuildProfile::RelWithDebInfo,
        _ => BuildProfile::Debug,
    };
    let build_config = cfg::BuildConfig {
        target: opt.target.clone(),
        profile,
        tool_info,
        features: opt.features.clone(),
    };
    let mut comps_config = cfg::ComponentsConfig {
        components: Vec::new(),
        sections: Vec::new(),
    };
    for spec in &opt.enable {
        let is_section_name = Section::ALL.iter().any(|s| s.plural() == spec);
        if is_section_name {
            comps_config.sections.push(spec.clone());
        } else {
            comps_config.components.push(spec.clone());
        }
    }
    let emit = cfg::EmitConfig {
        docker: if opt.docker {
            Some(cfg::DockerConfig {
                build_options: opt.docker_build_opt.clone(),
                tag: opt.docker_tag.clone(),
                write_tags_to_file: opt.docker_tags_log.clone(),
            })
        } else {
            None
        },
    };
    let config = cfg::Config {
        artifacts_dir: opt.out_dir.clone(),
        verbose: opt.verbose,
        emit,
        build: build_config,
        components: comps_config,
    };
    if std::env::var("CI").is_ok() {
        println!("Options: {:?}", &config);
    }
    let params = Params {
        cfg: config,
        src: jjs_src_path,
        build: opt.build_dir,
        out: opt.out_dir,
    };
    if let Err(err) = build_jjs_components(&params) {
        eprintln!("Error: {:#}", err);
        std::process::exit(1);
    }
}

pub struct Params {
    /// build config
    pub cfg: cfg::Config,
    /// jjs src dir
    pub src: PathBuf,
    /// jjs build dir
    pub build: PathBuf,
    /// output directory
    pub out: PathBuf,
}

fn make_rust_package_list() -> Vec<RustPackage> {
    let mut pkgs = Vec::new();
    let mut add = |pkg_name: &str, inst_name: &str, sec| {
        let pkg = RustPackage {
            name: pkg_name.to_string(),
            install_name: inst_name.to_string(),
            section: sec,
        };
        pkgs.push(pkg);
    };

    //add("cleanup", "jjs-cleanup", Section::Tool);
    //add("envck", "jjs-env-check", Section::Tool);
    //add("setup", "jjs-setup", Section::Tool);
    add("ppc", "jjs-ppc", Section::Tool);
    //add("userlist", "jjs-userlist", Section::Tool);
    //add("cli", "jjs-cli", Section::Tool);
    add("invoker", "jjs-invoker", Section::Daemon);
    //add("svaluer", "jjs-svaluer", Section::Suggested);
    /*add(
        "configure-toolchains",
        "jjs-configure-toolchains",
        Section::Tool,
    );*/

    pkgs
}

fn make_other_package_list() -> Vec<OtherPackage> {
    let mut pkgs = Vec::new();
    pkgs.push(OtherPackage {
        name: "apiserver".to_string(),
        section: Section::Daemon,
    });
    pkgs
}

fn make_cmake_package_list() -> Vec<CmakePackage> {
    let mut pkgs = Vec::new();
    pkgs.push(CmakePackage {
        name: "jtl".to_string(),
        section: Section::Tool,
    });
    pkgs
}

fn make_meta_package_list() -> Vec<MetaPackage> {
    let mut pkgs = Vec::new();
    pkgs.push(MetaPackage {
        name: "toolkit".to_string(),
        section: Section::Tool,
    });
    pkgs
}

fn check_filter(
    components_cfg: &crate::cfg::ComponentsConfig,
    pkg_name: &str,
    pkg_sect: Section,
) -> bool {
    let pkg_sect = pkg_sect.plural();
    components_cfg
        .sections
        .iter()
        .any(|enabled_section| enabled_section == pkg_sect)
        || components_cfg
            .components
            .iter()
            .any(|enabled_component| enabled_component == pkg_name)
}

fn check_meta_pkg_filter(components_cfg: &crate::cfg::ComponentsConfig, pkg_sect: Section) -> bool {
    let pkg_sect = pkg_sect.plural();
    components_cfg
        .sections
        .iter()
        .any(|enabled_section| enabled_section == pkg_sect)
}

/// Responsible for building of all requested components
fn build_jjs_components(params: &Params) -> anyhow::Result<()> {
    let rust_pkgs = make_rust_package_list()
        .into_iter()
        .filter(|pkg| check_filter(&params.cfg.components, &pkg.name, pkg.section))
        .collect::<Vec<_>>();
    let other_pkgs = make_other_package_list()
        .into_iter()
        .filter(|pkg| check_filter(&params.cfg.components, &pkg.name, pkg.section))
        .collect::<Vec<_>>();
    let cmake_pkgs = make_cmake_package_list()
        .into_iter()
        .filter(|pkg| check_filter(&params.cfg.components, &pkg.name, pkg.section))
        .collect::<Vec<_>>();
    let meta_pkgs = make_meta_package_list()
        .into_iter()
        .filter(|pkg| check_meta_pkg_filter(&params.cfg.components, pkg.section))
        .collect::<Vec<_>>();

    let mut builder = builder::Builder::new(params);
    for pkg in rust_pkgs {
        println!("Will build: {}", &pkg.name);
        builder.push_rust(pkg);
    }
    for pkg in cmake_pkgs {
        println!("Will build: {}", &pkg.name);
        builder.push_cmake(pkg);
    }
    let artifacts = builder.build().context("build error")?;

    if let Some(docker_cfg) = &params.cfg.emit.docker {
        let emitter = emit::DockerEmitter;
        emitter.emit(&artifacts, &other_pkgs, &meta_pkgs, params, docker_cfg)?;
    }
    Ok(())
}
