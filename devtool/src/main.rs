use std::{env, fs, process::Command};
use structopt::StructOpt;

#[derive(StructOpt)]
enum CliArgs {
    /// Create binary archive with all public components
    Pkg,
    /// Publish archive to Yandex.Drive (don't forget to run Pkg first)
    Publish,
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

fn task_package() {
    print_section("Building minion-cli");
    let st = Command::new(resolve_tool_path("cargo"))
        .current_dir(get_project_dir())
        .args(&[
            "build",
            "--bin",
            "minion-cli",
            "--release",
            "--target",
            "x86_64-unknown-linux-musl",
            "--features",
            "dist",
        ])
        .status()
        .unwrap()
        .success();
    assert_eq!(st, true);

    print_section("Building jjs-cleanup");

    let st = Command::new(resolve_tool_path("cargo"))
        .current_dir(get_project_dir())
        .args(&[
            "build",
            "--bin",
            "cleanup",
            "--release",
            "--target",
            "x86_64-unknown-linux-musl",
        ])
        .status()
        .unwrap()
        .success();
    assert_eq!(st, true);

    print_section("Building minion-ffi");
    let st = Command::new(resolve_tool_path("cargo"))
        .current_dir(get_project_dir())
        .args(&[
            "build",
            "--package",
            "minion-ffi",
            "--release",
            "--target",
            "x86_64-unknown-linux-musl",
        ])
        .status()
        .unwrap()
        .success();
    assert_eq!(st, true);
    let st = Command::new(resolve_tool_path("cargo"))
        .args(&[
            "build",
            "--package",
            "minion-ffi",
            "--release",
            "--target",
            "x86_64-unknown-linux-gnu",
        ])
        .status()
        .unwrap()
        .success();
    assert_eq!(st, true);
    print_section("Packaging[TGZ]");
    let binary_dir = format!(
        "{}/target/x86_64-unknown-linux-musl/release",
        get_project_dir()
    );
    let dylib_dir = format!(
        "{}/target/x86_64-unknown-linux-gnu/release",
        get_project_dir()
    );
    let pkg_dir = format!("{}/pkg/ar_data", get_project_dir());
    fs::remove_dir_all(&pkg_dir).ok();
    fs::create_dir(&pkg_dir).unwrap();
    fs::create_dir(format!("{}/lib", &pkg_dir)).ok();
    fs::create_dir(format!("{}/bin", &pkg_dir)).ok();
    fs::create_dir(format!("{}/include", &pkg_dir)).ok();
    fs::copy(
        format!("{}/libminion_ffi.so", &dylib_dir),
        format!("{}/lib/libminion_ffi.so", &pkg_dir),
    )
    .unwrap();
    fs::copy(
        format!("{}/libminion_ffi.a", &binary_dir),
        format!("{}/lib/libminion_ffi.a", &pkg_dir),
    )
    .unwrap();
    fs::copy(
        format!("{}/minion-cli", &binary_dir),
        format!("{}/bin/minion-cli", &pkg_dir),
    )
    .unwrap();
    fs::copy(
        format!("{}/target/minion-ffi.h", get_project_dir()),
        format!("{}/include/minion-ffi.h", &pkg_dir),
    )
    .unwrap();
    let st = Command::new("tar")
        .current_dir(get_project_dir())
        .args(&["cvzf", "pkg/jjs.tgz", "pkg/ar_data"])
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
    //dbg!(&upload_url);
    let tgz_pkg_path = format!("{}/pkg/jjs.tgz", get_project_dir());
    client
        .put(&upload_url)
        .header("Authorization", access_header.as_str())
        .body(fs::File::open(tgz_pkg_path).unwrap())
        .send()
        .unwrap()
        .text()
        .unwrap();
    //println!("{}",res);
}

fn main() {
    let args = CliArgs::from_args();
    match args {
        CliArgs::Pkg => task_package(),
        CliArgs::Publish => task_publish(),
    }
}
