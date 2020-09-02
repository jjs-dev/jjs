//! This package is responsible for generating various files useful
//! in distibution package.
use anyhow::Context as _;
use std::{ffi::OsStr, path::PathBuf, process::Command};
use structopt::StructOpt as _;
use util::cmd::CommandExt as _;

#[derive(structopt::StructOpt)]
struct Opts {
    /// Build testlib
    #[structopt(long)]
    testlib: bool,
    /// Build user manual
    #[structopt(long)]
    man: bool,
    /// Build api docs
    #[structopt(long)]
    apidoc: bool,
    /// Build rustdoc
    #[structopt(long)]
    rustdoc: bool,
    /// Generate env activate script
    #[structopt(long)]
    envscript: bool,
    /// Source dir
    #[structopt(long, default_value = ".")]
    source: PathBuf,
    /// Build dir
    #[structopt(long, default_value = "target")]
    build: PathBuf,
    /// Output dir
    #[structopt(long)]
    output: PathBuf,
    /// CMake build type (as in -DCMAKE_BUILD_TYPE=...)
    ///
    /// For example: Debug (default), Release, RelWithDebugInfo,
    #[structopt(long, default_value = "Debug")]
    cmake_build_type: String,
}

struct Params {
    source: PathBuf,
    build: PathBuf,
    output: PathBuf,
    cmake_build_type: String,
}

fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::from_args();
    let params = Params {
        source: opts.source.clone(),
        build: opts.build.clone(),
        cmake_build_type: opts.cmake_build_type.clone(),
        output: opts.output.clone(),
    };
    if opts.apidoc {
        generate_api_docs(&params)?;
    }
    if opts.testlib {
        build_testlib(&params)?;
    }
    if opts.man {
        generate_man(&params)?;
    }
    if opts.rustdoc {
        generate_rustdoc(&params)?;
    }
    if opts.envscript {
        generate_envscript(&params)?;
    }
    Ok(())
}

fn build_testlib(params: &Params) -> anyhow::Result<()> {
    println!("Build testlib[C++]");
    let jtl_path = params.source.join("jtl-cpp");
    let cmake_build_dir = params.build.join("jtl-cpp");
    std::fs::create_dir_all(&cmake_build_dir).ok();

    let mut cmd = Command::new("cmake");

    let mut cmake_arg_install_prefix = OsStr::new("-DCMAKE_INSTALL_PREFIX=").to_os_string();
    cmake_arg_install_prefix.push(&params.output);
    let mut cmake_arg_build_type = OsStr::new("-DCMAKE_BUILD_TYPE=").to_os_string();
    cmake_arg_build_type.push(&params.cmake_build_type);

    cmd.current_dir(&cmake_build_dir)
        .arg(&jtl_path)
        .arg(cmake_arg_install_prefix)
        .arg(cmake_arg_build_type);
    cmd.try_exec().context("cmake configure failed")?;

    Command::new("cmake")
        .arg("--build")
        .arg(&cmake_build_dir)
        .args(&["--target", "install"])
        .try_exec()
        .context("cmake build failed")?;
    Ok(())
}

fn generate_man(params: &Params) -> anyhow::Result<()> {
    println!("building man");
    let book_dir = params.source.join("man");
    Command::new("mdbook")
        .current_dir(&book_dir)
        .arg("build")
        .try_exec()
        .context("mdbook failed")?;
    println!("copying built man files");
    let opts = fs_extra::dir::CopyOptions {
        overwrite: true,
        skip_exist: false,
        buffer_size: 64 * 1024,
        copy_inside: true,
        depth: 0,
        content_only: true,
    };
    let src = book_dir.join("book");
    let dst = params.output.join("share/docs/man");
    std::fs::create_dir_all(&dst)?;
    fs_extra::dir::copy(&src, &dst, &opts)?;
    Ok(())
}

fn generate_api_docs(params: &Params) -> anyhow::Result<()> {
    if Command::new("npx")
        .arg("--help")
        .try_exec_with_output()
        .is_err()
    {
        anyhow::bail!("npx is not installed");
    }
    let schema_path = params
        .source
        .join("src/apiserver-engine/docs/openapi-gen.json");
    let docs_path = params.output.join("share/docs/api");
    Command::new("npx")
        .arg("@openapitools/openapi-generator-cli")
        .arg("generate")
        .arg("--input-spec")
        .arg(schema_path)
        .arg("--output")
        .arg(docs_path)
        .arg("--generator-name")
        .arg("html2")
        .try_exec()
        .context("failed to generate api docs")?;
    Ok(())
}

fn generate_rustdoc(params: &Params) -> anyhow::Result<()> {
    println!("Generating source code API docsumentation");
    Command::new("cargo")
        .arg("doc")
        .arg("--no-deps")
        .arg("--document-private-items")
        .try_exec()?;

    let src = params.build.join("doc");

    let dest = params.output.join("share/docs/rustdoc");
    std::fs::create_dir_all(&dest).unwrap();
    let opts = fs_extra::dir::CopyOptions {
        overwrite: true,
        skip_exist: false,
        buffer_size: 64 * 1024,
        copy_inside: true,
        depth: 0,
        content_only: true,
    };
    fs_extra::dir::copy(&src, &dest, &opts).unwrap();

    Ok(())
}

fn env_add(var_name: &str, prepend: &str) -> String {
    format!("export {}={}:${}", var_name, prepend, var_name)
}

fn generate_envscript(params: &Params) -> anyhow::Result<()> {
    use std::fmt::Write;
    println!("Generate environ activate script");

    let mut out = String::new();
    writeln!(out, "export JJS_PATH={}", params.output.display()).unwrap();
    writeln!(
        out,
        "{}",
        env_add("LIBRARY_PATH", &format!("{}/lib", params.output.display()),)
    )?;
    writeln!(
        out,
        "{}",
        env_add("PATH", &format!("{}/bin", params.output.display()))
    )?;
    writeln!(
        out,
        "{}",
        env_add(
            "CPLUS_INCLUDE_PATH",
            &format!("{}/include", params.output.display()),
        )
    )?;
    writeln!(
        out,
        "{}",
        env_add(
            "CMAKE_PREFIX_PATH",
            &format!("{}/share/cmake", params.output.display()),
        )
    )?;

    let out_file_path = params.output.join("share/env.sh");
    std::fs::write(&out_file_path, out)?;
    Ok(())
}
