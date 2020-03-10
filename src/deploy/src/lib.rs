mod build_ctx;
pub mod cfg;
mod deb;
mod docker;
mod inst_ctx;
mod packages;
mod pkg;
mod registry;
mod sel_ctx;
mod systemd;
pub mod util;

use crate::{
    build_ctx::BuildCtx,
    cfg::BuildProfile,
    inst_ctx::InstallCtx,
    packages::{BinPackage, BinPackages},
    pkg::PackageComponentKind,
    registry::Registry,
    sel_ctx::SelCtx,
    util::print_section,
};
use ::util::cmd::{CommandExt, Runner};
use std::{ffi::OsStr, fs, path::PathBuf, process::Command};

pub struct Params {
    /// build config
    pub cfg: cfg::Config,
    /// jjs src dir
    pub src: PathBuf,
    /// jjs build dir
    pub build: PathBuf,
    /// Intermediate sysroot dir (for compressing / copying), containing only build artifacts
    pub artifacts: PathBuf,
    /// Target installation dir, if given (only to generate some paths)
    pub install_prefix: Option<PathBuf>,
}

fn create_registry() -> Registry {
    let mut reg = Registry::new();
    let mut bin_pkgs = Vec::new();
    let mut add_bin = |pkg_name, inst_name, comp| {
        let pkg = BinPackage::new(pkg_name, inst_name, comp);
        bin_pkgs.push(pkg);
    };

    add_bin("cleanup", "jjs-cleanup", PackageComponentKind::Tools);
    add_bin("envck", "jjs-env-check", PackageComponentKind::Tools);
    add_bin("setup", "jjs-setup", PackageComponentKind::Tools);
    add_bin("ppc", "jjs-ppc", PackageComponentKind::Tools);
    add_bin("frontend", "jjs-frontend", PackageComponentKind::Core);
    add_bin("userlist", "jjs-userlist", PackageComponentKind::Tools);
    add_bin("cli", "jjs-cli", PackageComponentKind::Tools);
    add_bin("invoker", "jjs-invoker", PackageComponentKind::Core);
    add_bin("svaluer", "jjs-svaluer", PackageComponentKind::Core);
    add_bin(
        "soft",
        "jjs-configure-toolchains",
        PackageComponentKind::Tools,
    );
    add_bin("minion-cli", "jjs-minion-cli", PackageComponentKind::Extra);
    {
        let minion_ffi = packages::MinionFfiPackage::new();
        reg.add(minion_ffi);
    }
    reg.add(BinPackages::new(bin_pkgs));

    reg
}

fn build_jjs_components(params: &Params, runner: &Runner) {
    let proj_root = &params.src;

    print_section("Creating directories");
    let pkg_dir = params.artifacts.clone();

    util::make_empty(&pkg_dir).unwrap();
    fs::create_dir(pkg_dir.join("lib")).ok();
    fs::create_dir(pkg_dir.join("lib/systemd")).ok();
    fs::create_dir(pkg_dir.join("lib/systemd/system")).ok();
    fs::create_dir(pkg_dir.join("bin")).ok();
    fs::create_dir(pkg_dir.join("include")).ok();
    fs::create_dir(pkg_dir.join("share")).ok();
    fs::create_dir(pkg_dir.join("share/cmake")).ok();
    fs::create_dir(pkg_dir.join("pkg")).ok();

    let mut reg = create_registry();

    let sctx = SelCtx::new(params);
    let bctx = BuildCtx::new(params, runner);
    let ictx = InstallCtx::new(params);
    reg.run_selection(&sctx);
    reg.build(&bctx);
    print_section("Installing");
    reg.install(&ictx);

    print_section("Generating migration script");
    {
        let mut migration_script: Vec<_> = fs::read_dir(proj_root.join("src/db/migrations"))
            .unwrap()
            .map(|ent| ent.unwrap().path().to_str().unwrap().to_string())
            .filter(|x| !x.contains(".gitkeep"))
            .map(|x| format!("{}/up.sql", x))
            .collect();
        migration_script.sort();
        let migration_script = migration_script
            .into_iter()
            .map(|path| fs::read(path).unwrap())
            .map(|bytes| String::from_utf8(bytes).unwrap());
        let migration_script = migration_script.collect::<Vec<_>>().join("\n\n\n");
        let src_path = pkg_dir.join("share/db-setup.sql");
        fs::write(src_path, &migration_script).unwrap();
    }
    print_section("Copying files");
    let opts = fs_extra::dir::CopyOptions {
        overwrite: true,
        skip_exist: false,
        buffer_size: 64 * 1024,
        copy_inside: true,
        depth: 0,
    };

    let copy_dir = |dir_name: &str| {
        fs_extra::dir::copy(proj_root.join(dir_name), pkg_dir.join(dir_name), &opts).unwrap();
    };

    copy_dir("example-config");
    copy_dir("example-problems");
    copy_dir("toolchains");
}

pub fn package(params: &Params, runner: &Runner) {
    build_jjs_components(params, runner);
    if params.cfg.components.testlib {
        build_testlib(params);
    }
    if params.cfg.components.man {
        generate_man(params);
    }
    if params.cfg.components.api_doc {
        generate_api_docs(params);
    }
    if params.cfg.components.json_schema {
        generate_json_schema(params);
    }
    runner.exit_if_errors();

    generate_envscript(params);
    if params.cfg.packaging.systemd {
        print_section("Generating SystemD unit files");
        systemd::build(params);
    }
    if params.cfg.components.archive {
        generate_archive(params);
    }
    if let Some(opts) = &params.cfg.packaging.deb {
        print_section("Generating Debian package");
        deb::create(params, runner, opts);
    }
    if let Some(opts) = &params.cfg.packaging.docker {
        print_section("Building docker images");
        docker::build_docker_image(params, opts, runner);
    }
}

fn generate_archive(params: &Params) {
    print_section("Packaging[TGZ]");
    let out_file_path = params.build.join("jjs.tgz");
    let out_file =
        std::fs::File::create(&out_file_path).expect("couldn't open archive for writing");
    println!(
        "packaging {} into {}",
        params.artifacts.display(),
        &out_file_path.display()
    );
    let mut builder = tar::Builder::new(out_file);
    builder
        .append_dir_all("jjs", &params.artifacts)
        .expect("couldn't add files to archive");
    let _ = builder
        .into_inner()
        .expect("couldn't finish writing archive");
}

fn build_testlib(params: &Params) {
    let proj_dir = &params.src;
    print_section("Build testlib[C++]");
    let jtl_path = proj_dir.join("jtl-cpp");
    let cmake_build_dir = params.build.join("jtl-cpp");
    let sysroot_dir = &params.artifacts;
    util::ensure_exists(&cmake_build_dir).unwrap();
    let cmake_build_type = match params.cfg.build.profile {
        BuildProfile::Debug => "Debug",
        BuildProfile::Release => "Release",
        BuildProfile::RelWithDebInfo => "RelWithDebInfo",
    };
    let mut cmd = Command::new(&params.cfg.build.tool_info.cmake);

    let mut cmake_arg_install_prefix = OsStr::new("-DCMAKE_INSTALL_PREFIX=").to_os_string();
    cmake_arg_install_prefix.push(sysroot_dir);
    let mut cmake_arg_build_type = OsStr::new("-DCMAKE_BUILD_TYPE=").to_os_string();
    cmake_arg_build_type.push(cmake_build_type);

    cmd.current_dir(&cmake_build_dir)
        .arg(&jtl_path)
        .arg(cmake_arg_install_prefix)
        .arg(cmake_arg_build_type);

    if params.cfg.verbose {
        cmd.arg("-DCMAKE_VERBOSE_MAKEFILE=On");
    }

    let st = cmd.status().unwrap();

    assert!(st.success());

    let st = Command::new(&params.cfg.build.tool_info.cmake)
        .arg("--build")
        .arg(&cmake_build_dir)
        .args(&["--target", "install"])
        .status()
        .unwrap();
    assert!(st.success());
}

fn generate_api_docs(params: &Params) {
    print_section("building graphql docs");
    let mut cmd = Command::new("graphdoc");
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(params.src.join("src/frontend-api/src/schema-gen.json")).unwrap(),
    )
    .unwrap();
    let schema =
        serde_json::Value::Object(vec![("data".to_string(), schema)].into_iter().collect());
    let hacked_schema_path = params.build.join("schema-gen-fixed.json");
    let schema = serde_json::to_string(&schema).unwrap();
    std::fs::write(&hacked_schema_path, schema).unwrap();
    cmd.arg("--schema-file").arg(&hacked_schema_path);
    cmd.arg("--output")
        .arg(params.artifacts.join("share/docs/api"));
    cmd.try_exec().unwrap();
}

fn generate_man(params: &Params) {
    print_section("building man");
    let book_dir = params.src.join("man");
    let st = Command::new("mdbook")
        .current_dir(&book_dir)
        .arg("build")
        .status()
        .unwrap()
        .success();
    assert_eq!(st, true);
    print_section("copying built man files");
    let opts = fs_extra::dir::CopyOptions {
        overwrite: true,
        skip_exist: false,
        buffer_size: 64 * 1024,
        copy_inside: true,
        depth: 0,
    };
    let src = book_dir.join("book");
    let src = fs::read_dir(src)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let dst = params.artifacts.join("share/docs/man");
    fs::create_dir_all(&dst).unwrap();
    fs_extra::copy_items(&src, &dst, &opts).unwrap();

    assert_eq!(st, true);
}

fn generate_json_schema(params: &Params) {
    print_section("Generating json schemas");
    let out_dir = params.artifacts.join("share/schema");
    fs::create_dir_all(&out_dir).unwrap();
    let bin_out_dir = params.artifacts.join("bin");
    let frontend_bin = bin_out_dir.join("jjs-frontend");
    let frontend_out = Command::new(frontend_bin)
        .env("__JJS_SPEC", "config-schema")
        .output()
        .expect("failed to invoke jjs-frontend");
    assert!(frontend_out.status.success());
    fs::write(out_dir.join("frontend-config.json"), frontend_out.stdout)
        .expect("failed to write schema");
}

fn env_add(var_name: &str, prepend: &str) -> String {
    format!("export {}={}:${}", var_name, prepend, var_name)
}

fn generate_envscript(params: &Params) {
    use std::fmt::Write;
    print_section("Generate environ activate script");
    if let Some(install_prefix) = &params.install_prefix {
        let mut out = String::new();
        writeln!(out, "export JJS_PATH={}", &install_prefix.display()).unwrap();
        writeln!(
            out,
            "{}",
            env_add(
                "LIBRARY_PATH",
                &format!("{}/lib", &install_prefix.display()),
            )
        )
        .unwrap();
        writeln!(
            out,
            "{}",
            env_add("PATH", &format!("{}/bin", &install_prefix.display()))
        )
        .unwrap();
        writeln!(
            out,
            "{}",
            env_add(
                "CPLUS_INCLUDE_PATH",
                &format!("{}/include", &install_prefix.display()),
            )
        )
        .unwrap();
        writeln!(
            out,
            "{}",
            env_add(
                "CMAKE_PREFIX_PATH",
                &format!("{}/share/cmake", &install_prefix.display()),
            )
        )
        .unwrap();

        let out_file_path = params.artifacts.join("share/env.sh");
        std::fs::write(&out_file_path, out).unwrap();
    } else {
        eprintln!(
            "warning: skipping generating environment vars activation script because --install-prefix missing"
        );
    }
}
