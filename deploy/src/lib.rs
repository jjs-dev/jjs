mod build_ctx;
pub mod cfg;
mod inst_ctx;
mod packages;
mod pkg;
mod registry;
mod sel_ctx;
pub mod util;

use crate::{
    build_ctx::BuildCtx, cfg::BuildProfile, inst_ctx::InstallCtx, packages::BinPackage,
    pkg::PackageComponentKind, registry::Registry, sel_ctx::SelCtx,
};
use std::{fs, process::Command};
use util::print_section;

pub struct Params {
    // build config
    pub cfg: cfg::Config,
    // jjs src dir
    pub src: String,
    // jjs build dir
    pub build: String,
    // Intermediate sysroot dir (for compressing / copying), containing only build artifacts
    pub sysroot: String,
}

fn create_registry() -> Registry {
    let mut reg = Registry::new();

    let mut add_bin = |pkg_name, inst_name, comp| {
        let pkg = BinPackage::new(pkg_name, inst_name, comp);
        reg.add(pkg);
    };

    add_bin("cleanup", "jjs-cleanup", PackageComponentKind::Tools);
    add_bin("envck", "jjs-env-check", PackageComponentKind::Tools);
    add_bin("init-jjs-root", "jjs-mkroot", PackageComponentKind::Tools);
    add_bin("ppc", "jjs-ppc", PackageComponentKind::Tools);
    add_bin("userlist", "jjs-userlist", PackageComponentKind::Tools);
    add_bin("invoker", "jjs-invoker", PackageComponentKind::Core);
    add_bin("frontend", "jjs-frontend", PackageComponentKind::Core);
    add_bin("cli", "jjs-cli", PackageComponentKind::Tools);

    {
        let mut minion_cli =
            packages::BinPackage::new("minion-cli", "jjs-minion-cli", PackageComponentKind::Extra);
        minion_cli.feature("dist");

        reg.add(minion_cli);
    }
    {
        let minion_ffi = packages::MinionFfiPackage::new();
        reg.add(minion_ffi);
    }

    reg
}

fn build_jjs_components(params: &Params) {
    let proj_root = &params.src;

    print_section("Creating directories");
    let pkg_dir = params.sysroot.clone();

    util::make_empty(&pkg_dir).unwrap();
    fs::create_dir(format!("{}/lib", &pkg_dir)).ok();
    fs::create_dir(format!("{}/bin", &pkg_dir)).ok();
    fs::create_dir(format!("{}/include", &pkg_dir)).ok();
    fs::create_dir(format!("{}/share", &pkg_dir)).ok();
    fs::create_dir(format!("{}/share/cmake", &pkg_dir)).ok();

    let mut reg = create_registry();

    let sctx = SelCtx::new(params);
    let bctx = BuildCtx::new(params);
    let ictx = InstallCtx::new(params);
    reg.run_selection(&sctx);
    reg.build(&bctx);
    print_section("Installing");
    reg.install(&ictx);

    print_section("Generating migration script");
    {
        let mut migration_script: Vec<_> = fs::read_dir(format!("{}/db/migrations", &proj_root))
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
        let src_path = format!("{}/share/db-setup.sql", &pkg_dir);
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
    fs_extra::dir::copy(
        format!("{}/example-config", &proj_root),
        format!("{}/example-config", &pkg_dir),
        &opts,
    )
    .unwrap();
}

pub fn package(params: &Params) {
    build_jjs_components(params);
    if params.cfg.components.testlib {
        build_testlib(params);
    }
    if params.cfg.components.man {
        generate_man(params);
    }
    if params.cfg.components.archive {
        generate_archive(params);
    }

    generate_envscript(params);
}

fn generate_archive(params: &Params) {
    print_section("Packaging[TGZ]");
    let out_file_path = format!("{}/jjs.tgz", &params.build);
    let out_file =
        std::fs::File::create(&out_file_path).expect("couldn't open archive for writing");
    println!("packaging {} into {}", &params.sysroot, &out_file_path);
    let mut builder = tar::Builder::new(out_file);
    builder
        .append_dir_all("jjs", &params.sysroot)
        .expect("couldn't add files to archive");
    let _ = builder
        .into_inner()
        .expect("couldn't finish writing archive");
}

fn build_testlib(params: &Params) {
    let proj_dir = &params.src;
    print_section("Build testlib[C++]");
    let jtl_path = format!("{}/jtl-cpp", &proj_dir);
    let cmake_build_dir = format!("{}/jtl-cpp/cmake-build-debug", &proj_dir);
    let sysroot_dir = params.sysroot.clone();
    util::ensure_exists(&cmake_build_dir).unwrap();
    let cmake_build_type = match params.cfg.build.profile {
        BuildProfile::Debug => "Debug",
        BuildProfile::Release => "Release",
        BuildProfile::RelWithDebInfo => "RelWithDebInfo",
    };
    let mut cmd = Command::new(&params.cfg.build.tool_info.cmake);
    cmd.current_dir(&cmake_build_dir)
        .arg(&jtl_path)
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", &sysroot_dir))
        .arg(format!("-DCMAKE_BUILD_TYPE={}", cmake_build_type));

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

fn generate_man(params: &Params) {
    print_section("building docs");
    let book_dir = format!("{}/man", &params.src);
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
    let src = format!("{}/man/book", &params.src);
    let src = fs::read_dir(src)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let dst = format!("/{}/share/docs", &params.sysroot);
    fs::create_dir_all(&dst).unwrap();
    fs_extra::copy_items(&src, &dst, &opts).unwrap();

    assert_eq!(st, true);
}

fn env_add(var_name: &str, prepend: &str) -> String {
    format!("export {}={}:${}", var_name, prepend, var_name)
}

fn generate_envscript(params: &Params) {
    print_section("Generate environ activate script");
    use std::fmt::Write;
    let mut out = String::new();
    writeln!(out, "export JJS_PATH={}", &params.sysroot).unwrap();
    writeln!(
        out,
        "{}",
        env_add("LIBRARY_PATH", &format!("{}/lib", &params.sysroot))
    )
    .unwrap();
    writeln!(
        out,
        "{}",
        env_add("PATH", &format!("{}/bin", &params.sysroot))
    )
    .unwrap();
    writeln!(
        out,
        "{}",
        env_add(
            "CPLUS_INCLUDE_PATH",
            &format!("{}/include", &params.sysroot),
        )
    )
    .unwrap();
    writeln!(
        out,
        "{}",
        env_add(
            "CMAKE_PREFIX_PATH",
            &format!("{}/share/cmake", &params.sysroot),
        )
    )
    .unwrap();
    let out_file_path = format!("{}/share/env.sh", &params.sysroot);
    std::fs::write(&out_file_path, out).unwrap();
}
