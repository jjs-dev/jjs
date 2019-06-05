use deploy::cfg;

use serde::ser::Serialize;
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    /// Build directory
    #[structopt(short = "I", long = "out-dir", default_value = "target")]
    build_dir: String,
    /// Build and install testlib
    #[structopt(long = "enable-testlib")]
    testlib: bool,
    /// Build and install manual
    #[structopt(long = "enable-man")]
    man: bool,
    /// Generate tarball
    #[structopt(long = "enable-archive")]
    archive: bool,
    /// Cargo path
    #[structopt(long = "with-cargo")]
    cargo: Option<String>,
    /// CMake path
    #[structopt(long = "with-cmake")]
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
    /// Prefix
    #[structopt(long = "prefix", short = "P")]
    install_prefix: Option<String>,
}

static MAKE_SCRIPT_TPL: &str = include_str!("../make-tpl.sh");

fn check_cwd() {
    let cwd = std::env::current_dir().unwrap();
    let manif = cwd.join("Cargo.toml");
    let data = std::fs::read(manif).unwrap();
    let data = String::from_utf8(data).unwrap();
    if !data.contains("workspace") {
        eprintln!("Current dir is not JJS src root");
        std::process::exit(1);
    }
}

fn generate_make_script(opt: &Opt) {
    use std::os::unix::fs::PermissionsExt;
    let mut substitutions = HashMap::new();
    substitutions.insert("BUILD_DIR", opt.build_dir.clone());
    let current_dir = std::env::current_dir().unwrap().canonicalize().unwrap();
    substitutions.insert("SRC_DIR", current_dir.display().to_string());
    let mut subst_text = String::new();
    for (k, v) in substitutions {
        let v_esc = shell_escape::escape(v.into());
        let line = format!("export JJS_{}=\"{}\"\n", k, &v_esc);
        subst_text.push_str(&line);
    }
    let script = MAKE_SCRIPT_TPL.replace("$SUBST$", &subst_text);
    let script_path = format!("{}/make", &opt.build_dir);
    std::fs::write(&script_path, script).unwrap();
    let perms = std::fs::Permissions::from_mode(0o744);
    std::fs::set_permissions(&script_path, perms).unwrap();
}

fn main() {
    check_cwd();
    let opt: Opt = Opt::from_args();
    let tool_info = cfg::ToolInfo {
        cargo: opt
            .cargo
            .as_ref()
            .map(String::as_str)
            .unwrap_or_else(|| "cargo")
            .to_string(),
        cmake: opt
            .cmake
            .as_ref()
            .map(String::as_str)
            .unwrap_or_else(|| "cmake")
            .to_string(),
    };
    let profile = match (opt.optimize, opt.dbg_sym) {
        (true, false) => cfg::BuildProfile::Release,
        (true, true) => cfg::BuildProfile::RelWithDebInfo,
        _ => cfg::BuildProfile::Debug,
    };
    let build_config = cfg::Config {
        prefix: opt.install_prefix.clone(),
        target: match &opt.target {
            Some(t) => t.clone(),
            None => deploy::util::get_current_target(),
        },
        profile,
        man: opt.man,
        testlib: opt.testlib,
        tool_info,
        archive: opt.archive,
    };
    let manifest_path = format!("{}/jjs-build-config.json", &opt.build_dir);
    println!("Configuration: {}", &build_config);
    println!("Emitting JJS build config: {}", &manifest_path);
    let out_file = std::fs::File::create(&manifest_path).unwrap();
    let mut ser = serde_json::Serializer::pretty(out_file);
    build_config.serialize(&mut ser).unwrap();
    generate_make_script(&opt);
}
