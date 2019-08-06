mod util;

use std::{fs, process::Command};
use structopt::StructOpt;
use util::get_project_dir;

#[derive(StructOpt)]
enum CliArgs {
    /// Helper command to setup VM with jjs
    #[structopt(name = "vm")]
    Vm,
    /// Lint project
    #[structopt(name = "build")]
    Build,
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

trait CommandExt {
    fn run_check_status(&mut self);
}

impl CommandExt for Command {
    fn run_check_status(&mut self) {
        let st = self.status().unwrap();
        assert!(st.success());
    }
}

fn task_build() {
    Command::new("cargo")
        .args(&["fmt", "--verbose", "--all", "--", "--check"])
        .run_check_status();

    Command::new("cargo")
        .args(&[
            "clippy",
            "--all",
            "--",
            "-D",
            "clippy::all",
            "-D",
            "warnings",
        ])
        .run_check_status();
}

fn main() {
    let args = CliArgs::from_args();
    match args {
        CliArgs::Vm => task_vm(),
        CliArgs::Build => task_build(),
    }
}
