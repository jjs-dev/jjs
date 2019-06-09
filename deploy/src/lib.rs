pub mod cfg;
pub mod util;

use cfg::BuildProfile;
use std::{fs, process::Command};
use util::print_section;

pub struct Params {
    // build config
    pub cfg: cfg::Config,
    // jjs src dir
    pub src: String,
    // jjs build dir
    pub build: String,
    // Intermediate sysroot dir (for gzipping / copying)
    pub sysroot: String,
}

#[derive(Copy, Clone)]
struct BinaryArtifactAdder<'a> {
    pkg_dir: &'a str,
    params: &'a Params,
}

impl<'a> BinaryArtifactAdder<'a> {
    fn add(&self, build_name: &str, inst_name: &str) -> &Self {
        let comp = match self.params.cfg.profile {
            BuildProfile::Debug => "debug",
            BuildProfile::Release | BuildProfile::RelWithDebInfo => "release",
        };
        let binary_dir = format!(
            "{}/target/x86_64-unknown-linux-gnu/{}",
            &self.params.src, comp,
        );
        fs::copy(
            format!("{}/{}", &binary_dir, build_name),
            format!("{}/bin/{}", &self.pkg_dir, inst_name),
        )
            .unwrap();

        self
    }
}

#[derive(Copy, Clone)]
struct PackageBuilder<'a> {
    params: &'a Params,
}

impl<'a> PackageBuilder<'a> {
    fn build(&self, pkg_name: &str, features: &[&str]) -> &Self {
        print_section(&format!("Building {}", pkg_name));
        let mut cmd = Command::new(&self.params.cfg.tool_info.cargo);
        cmd.current_dir(&self.params.src).args(&[
            "build",
            "--package",
            pkg_name,
            "--target",
            &self.params.cfg.target,
        ]);
        if !features.is_empty() {
            cmd.arg("--features");
            let feat = features.join(",");
            cmd.arg(&feat);
        }
        let profile = self.params.cfg.profile;
        if let BuildProfile::Release | BuildProfile::RelWithDebInfo = profile {
            cmd.arg("--release");
        }
        if let BuildProfile::RelWithDebInfo = profile {
            cmd.env("CARGO_PROFILE_RELEASE_DEBUG", "true")
                .args(&["-Z", "config-profile"]);
        }
        let st = cmd.status().unwrap().success();
        assert_eq!(st, true);
        self
    }
}

struct SimpleBuilder<'a> {
    params: &'a Params,
    pkg_builder: PackageBuilder<'a>,
    art_adder: BinaryArtifactAdder<'a>,
}

impl<'a> SimpleBuilder<'a> {
    fn build(&self, pkg_name: &str, inst_name: &str, cond: bool) -> &Self {
        if cond {
            self.pkg_builder.build(pkg_name, &[]);
            self.art_adder.add(pkg_name, inst_name);
        }
        self
    }
}

fn build_jjs_components(params: &Params) {
    let target = &params.cfg.target;
    let enabled_dll = !target.contains("musl");
    let proj_root = &params.src;

    print_section("Creating directories");
    let binary_dir = format!("{}/target/{}/release", proj_root, &target);
    let dylib_dir = format!("{}/target/{}/release", proj_root, &target);
    let pkg_dir = params.sysroot.clone();

    util::make_empty(&pkg_dir).unwrap();
    fs::create_dir(format!("{}/lib", &pkg_dir)).ok();
    fs::create_dir(format!("{}/bin", &pkg_dir)).ok();
    fs::create_dir(format!("{}/include", &pkg_dir)).ok();
    fs::create_dir(format!("{}/share", &pkg_dir)).ok();

    let package_builder = PackageBuilder { params };
    let artifact_adder = BinaryArtifactAdder {
        pkg_dir: &pkg_dir,
        params,
    };
    let simple = SimpleBuilder {
        params,
        pkg_builder: package_builder,
        art_adder: artifact_adder,
    };
    simple
        .build("cleanup", "jjs-cleanup", params.cfg.tools)
        .build("envck", "jjs-env-check", params.cfg.tools)
        .build("init-jjs-root", "jjs-mkroot", true)
        .build("tt", "jjs-tt", params.cfg.tools)
        .build("userlist", "jjs-userlist", params.cfg.tools)
        .build("invoker", "jjs-invoker", true)
        .build("frontend", "jjs-frontend", true)
        .build("mgr", "jjs-mgr", params.cfg.tools);
    package_builder.build("minion-cli", &["dist"]);

    print_section("Building minion-ffi");
    let st = Command::new(&params.cfg.tool_info.cargo)
        .current_dir(&proj_root)
        .args(&[
            "build",
            "--package",
            "minion-ffi",
            "--release",
            "--target",
            &target,
        ])
        .status()
        .unwrap()
        .success();
    assert_eq!(st, true);

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

    if enabled_dll {
        fs::copy(
            format!("{}/libminion_ffi.so", &dylib_dir),
            format!("{}/lib/libminion_ffi.so", &pkg_dir),
        )
            .unwrap();
    }

    fs::copy(
        format!("{}/libminion_ffi.a", &binary_dir),
        format!("{}/lib/libminion_ffi.a", &pkg_dir),
    )
        .unwrap();

    artifact_adder.add("minion-cli", "jjs-minion-cli");
    fs::copy(
        format!("{}/target/minion-ffi.h", &proj_root),
        format!("{}/include/minion-ffi.h", &pkg_dir),
    )
        .unwrap();
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
    if params.cfg.testlib {
        build_testlib(params);
    }
    if params.cfg.archive {
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
    let cmake_build_dir = format!("{}/target/jtl-cpp", &proj_dir);
    let sysroot_dir = params.sysroot.clone();
    util::ensure_exists(&cmake_build_dir).unwrap();
    let st = Command::new(&params.cfg.tool_info.cmake)
        .current_dir(&cmake_build_dir)
        .arg(&jtl_path)
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", &sysroot_dir))
        .status()
        .unwrap();

    assert!(st.success());

    let st = Command::new(&params.cfg.tool_info.cmake)
        .arg("--build")
        .arg(&cmake_build_dir)
        .args(&["--target", "install"])
        .status()
        .unwrap();
    assert!(st.success());
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
    let out_file_path = format!("{}/share/env.sh", &params.sysroot);
    std::fs::write(&out_file_path, out).unwrap();
}
