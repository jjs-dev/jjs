use std::{env, fs, process::Command};
use structopt::StructOpt;

#[derive(StructOpt)]
struct TouchArgs {
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
}

#[derive(Copy, Clone, Debug)]
enum BuildProfile {
    Debug,
    Release,
    RelWithDebInfo,
}

impl std::str::FromStr for BuildProfile {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(BuildProfile::Debug),
            "release" => Ok(BuildProfile::Release),
            "release-dbg" => Ok(BuildProfile::RelWithDebInfo),
            _ => Err(format!("unknown profile: '{}'", s)),
        }
    }
}

#[derive(StructOpt)]
struct PackageArgs {
    #[structopt(short = "t", long = "target")]
    target: Option<String>,
    #[structopt(short = "p", long = "profile")]
    profile: String,
}

#[derive(StructOpt)]
enum CliArgs {
    /// Create binary archive with all public components
    #[structopt(name = "pkg")]
    Pkg(PackageArgs),
    /// Publish archive to Yandex.Drive (don't forget to run Pkg first)
    #[structopt(name = "publish")]
    Publish,
    /// Build man and publish to Github Pages
    #[structopt(name = "man")]
    Man,
    /// Helper command to setup VM with jjs
    #[structopt(name = "vm")]
    Vm,
    /// Touch all crates in workspace, so cargo-check or clippy will lint them
    #[structopt(name = "touch")]
    Touch(TouchArgs),
}

fn get_primary_style() -> console::Style {
    console::Style::new().green()
}

fn print_section(section: &str) {
    let msg = format!("----> {}", section);
    println!("{}", get_primary_style().apply_to(msg));
}

fn resolve_tool_path(toolname: &str) -> String {
    //TODO
    toolname.into()
}

fn get_project_dir() -> String {
    let mut path = std::env::current_dir().unwrap();
    loop {
        let mut manifest_path = path.clone();
        manifest_path.push("Cargo.toml");
        match fs::read(manifest_path) {
            Ok(content) => {
                let content = String::from_utf8_lossy(&content);
                if content.contains("workspace") {
                    return path.to_str().unwrap().to_string();
                }
            }
            Err(_e) => (),
        }
        path = path
            .parent()
            .expect("JJS dir not found. Have you launched devtool inside source tree?")
            .into()
    }
}

fn add_binary_artifact(build_name: &str, inst_name: &str, profile: BuildProfile) {
    let comp = match profile {
        BuildProfile::Debug => "debug",
        BuildProfile::Release | BuildProfile::RelWithDebInfo => "release",
    };
    let binary_dir = format!(
        "{}/target/x86_64-unknown-linux-gnu/{}",
        get_project_dir(),
        comp,
    );
    let pkg_dir = format!("{}/pkg/ar_data", get_project_dir());
    fs::copy(
        format!("{}/{}", &binary_dir, build_name),
        format!("{}/bin/{}", &pkg_dir, inst_name),
    )
    .unwrap();
}

fn build_package(pkg_name: &str, features: &[&str], target: &str, profile: BuildProfile) {
    print_section(&format!("Building {}", pkg_name));
    let mut cmd = Command::new(resolve_tool_path("cargo"));
    cmd.current_dir(get_project_dir())
        .args(&["build", "--package", pkg_name, "--target", target]);
    if !features.is_empty() {
        cmd.arg("--features");
        let feat = features.join(",");
        cmd.arg(&feat);
    }
    if let BuildProfile::Release | BuildProfile::RelWithDebInfo = profile {
        cmd.arg("--release");
    }
    if let BuildProfile::RelWithDebInfo = profile {
        cmd.env("CARGO_PROFILE_RELEASE_DEBUG", "true").args(&["-Z", "config-profile"]);
    }
    let st = cmd.status().unwrap().success();
    assert_eq!(st, true);
}

fn get_current_target() -> String {
    //provided by build.rs
    env!("TARGET").to_owned()
}

fn task_package(args: PackageArgs) {
    let target = args.target.unwrap_or_else(get_current_target);
    let enabled_dll = !target.contains("musl");
    let build_profile: BuildProfile = args.profile.parse().expect("invalid build profile");

    print_section("Creating directories");
    let binary_dir = format!("{}/target/{}/release", get_project_dir(), &target);
    let dylib_dir = format!("{}/target/{}/release", get_project_dir(), &target);
    let pkg_dir = format!("{}/pkg/ar_data", get_project_dir());

    fs::create_dir_all(&pkg_dir).ok();
    fs::remove_dir_all(&pkg_dir).ok();
    fs::create_dir(&pkg_dir).unwrap();
    fs::create_dir(format!("{}/lib", &pkg_dir)).ok();
    fs::create_dir(format!("{}/bin", &pkg_dir)).ok();
    fs::create_dir(format!("{}/include", &pkg_dir)).ok();
    fs::create_dir(format!("{}/share", &pkg_dir)).ok();

    build_package("cleanup", &[], &target, build_profile);
    build_package("envck", &[], &target, build_profile);
    build_package("init-jjs-root", &[], &target, build_profile);
    build_package("minion-cli", &["dist"], &target, build_profile);
    build_package("userlist", &[], &target, build_profile);
    build_package("invoker", &[], &target, build_profile);
    build_package("frontend", &[], &target, build_profile);

    print_section("Building minion-ffi");
    let st = Command::new(resolve_tool_path("cargo"))
        .current_dir(get_project_dir())
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
        let mut migration_script: Vec<_> =
            fs::read_dir(format!("{}/db/migrations", get_project_dir()))
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
        let src_path = format!("{}/pkg/ar_data/share/db-setup.sql", get_project_dir());
        fs::write(src_path, &migration_script).unwrap();
    }
    print_section("Packaging[TGZ]");

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
    add_binary_artifact("minion-cli", "jjs-minion-cli", build_profile);
    add_binary_artifact("frontend", "jjs-frontend", build_profile);
    add_binary_artifact("invoker", "jjs-invoker", build_profile);
    add_binary_artifact("envck", "jjs-env-check", build_profile);
    add_binary_artifact("userlist", "jjs-userlist", build_profile);
    fs::copy(
        format!("{}/target/minion-ffi.h", get_project_dir()),
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
        format!("{}/example-config", get_project_dir()),
        format!("{}/example-config", &pkg_dir),
        &opts,
    )
    .unwrap();

    let st = Command::new("tar")
        .current_dir(format!("{}/pkg", get_project_dir()))
        .args(&["cvzf", "jjs.tgz", "-C", "ar_data", "."])
        .status()
        .unwrap()
        .success();

    assert_eq!(st, true);
}

fn task_publish() {
    let client = reqwest::Client::new();
    let access_token =
        env::var("JJS_DEVTOOL_YANDEXDRIVE_ACCESS_TOKEN").expect("access token not provided");
    let access_header = format!("OAuth {}", access_token);
    let upload_url = {
        let upload_path = "/jjs-dist/jjs.tgz";
        let upload_path = percent_encoding::percent_encode(
            upload_path.as_bytes(),
            percent_encoding::DEFAULT_ENCODE_SET,
        )
        .to_string();
        let req_url = format!(
            "https://cloud-api.yandex.net/v1/disk/resources/upload?path={}&overwrite=true",
            upload_path
        );
        let response: serde_json::Value = client
            .get(&req_url)
            .header("Authorization", access_header.as_str())
            .send()
            .unwrap()
            .json()
            .unwrap();
        response["href"].as_str().unwrap().to_string()
    };
    let tgz_pkg_path = format!("{}/pkg/jjs.tgz", get_project_dir());
    client
        .put(&upload_url)
        .header("Authorization", access_header.as_str())
        .body(fs::File::open(tgz_pkg_path).unwrap())
        .send()
        .unwrap()
        .text()
        .unwrap();
}

fn task_man() {
    print_section("building docs");
    let book_dir = format!("{}/man", get_project_dir());
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
    let src = format!("{}/man/book", get_project_dir());
    let src = fs::read_dir(src)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let dst = "/tmp/jjs-pages/manual";
    fs::create_dir_all(dst).unwrap();
    fs_extra::copy_items(&src, dst, &opts).unwrap();
    print_section("pushing pages");
    let helper_script_path = format!("{}/devtool/scripts/pages-push.sh", get_project_dir());
    let st = Command::new("bash")
        .current_dir("/tmp/jjs-pages")
        .args(&[&helper_script_path])
        .status()
        .unwrap()
        .success();

    assert_eq!(st, true);
}

fn task_vm() {
    let addr = "0.0.0.0:4567";
    println!("address: {}", addr);
    let setup_script_path = format!("{}/devtool/scripts/vm-setup.sh", get_project_dir());
    let pkg_path = format!("{}/pkg/jjs.tgz", get_project_dir());
    let pg_start_script_path = format!("{}/devtool/scripts/postgres-start.sh", get_project_dir());
    rouille::start_server(addr, move |request| {
        let url = request.url();
        if url == "/setup" {
            return rouille::Response::from_file(
                "text/x-shellscript",
                fs::File::open(&setup_script_path).unwrap(),
            );
        } else if url == "/pkg" {
            return rouille::Response::from_file(
                "application/gzip",
                fs::File::open(&pkg_path).unwrap(),
            );
        } else if url == "/pg-start" {
            return rouille::Response::from_file(
                "text/x-shellscript",
                fs::File::open(&pg_start_script_path).unwrap(),
            );
        }

        rouille::Response::from_data("text/plain", "ERROR: NOT FOUND")
    });
}

fn task_touch(arg: TouchArgs) {
    let workspace_root = get_project_dir();
    let items = fs::read_dir(workspace_root).expect("couldn't list dir");
    //let mut roots = Vec::new();
    for item in items {
        let info = item.expect("couldn't describe item");
        let item_type = info.file_type().expect("couldn't get item type");
        if !item_type.is_dir() {
            continue;
        }
        let path = info
            .file_name()
            .to_str()
            .expect("couldn't decode item path")
            .to_owned();
        // TODO: touch bin/*
        for root in &["src/main.rs", "src/lib.rs", "build.rs"] {
            let p = format!("{}/{}", &path, root);
            if std::fs::metadata(&p).is_ok() {
                if arg.verbose {
                    println!("touching {}", &p);
                }
                let time = filetime::FileTime::from_system_time(std::time::SystemTime::now());
                filetime::set_file_times(&p, time, time).expect("couldn't touch");
            }
        }
    }
}

fn main() {
    let args = CliArgs::from_args();
    match args {
        CliArgs::Pkg(args) => task_package(args),
        CliArgs::Publish => task_publish(),
        CliArgs::Man => task_man(),
        CliArgs::Vm => task_vm(),
        CliArgs::Touch(arg) => task_touch(arg),
    }
}
