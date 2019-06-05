mod util;

use std::{env, fs, process::Command};
use structopt::StructOpt;
use util::get_project_dir;

#[derive(StructOpt)]
struct TouchArgs {
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
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
        // CliArgs::Pkg(args) => task_package(args),
        //CliArgs::Publish => task_publish(),
        //CliArgs::Man => task_man(),
        CliArgs::Vm => task_vm(),
        CliArgs::Touch(arg) => task_touch(arg),
        //CliArgs::Testlib => task_testlib(),
    }
}
