use log::{debug, error, info};
use std::{
    env::set_current_dir,
    path::PathBuf,
    process::{exit, Command},
    sync::atomic::{AtomicBool, Ordering},
};
use structopt::StructOpt;

#[derive(StructOpt)]
enum CliArgs {
    /// Lint project
    #[structopt(name = "check")]
    Check,
    /// Run all tests
    #[structopt(name = "test")]
    Test,
}

static HAD_ERRORS: AtomicBool = AtomicBool::new(false);

trait CommandExt {
    fn run_check_status(&mut self);
}

impl CommandExt for Command {
    fn run_check_status(&mut self) {
        let st = self.status().unwrap();
        if !st.success() {
            error!("child command failed");
            HAD_ERRORS.store(true, Ordering::SeqCst);
        }
    }
}

fn find_scripts() -> impl Iterator<Item = PathBuf> {
    let mut types_builder = ignore::types::TypesBuilder::new();
    types_builder.add_defaults();
    types_builder.negate("all");
    types_builder.select("sh");
    let types_matched = types_builder.build().unwrap();
    ignore::WalkBuilder::new(".")
        .types(types_matched)
        .build()
        .map(Result::unwrap)
        .filter(|x| {
            let ty = x.file_type();
            match ty {
                Some(f) => f.is_file(),
                None => false,
            }
        })
        .map(|x| x.path().to_path_buf())
}

fn task_check() {
    info!("running cargo fmt --check");
    Command::new("cargo")
        .args(&["fmt", "--verbose", "--all", "--", "--check"])
        .run_check_status();

    info!("running clippy");
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

    let scripts = find_scripts().collect::<Vec<_>>();
    for script_chunk in scripts.chunks(10) {
        let mut cmd = Command::new("shellcheck");
        cmd.arg("--color=always");
        // FIXME: cmd.arg("--check-sourced");
        // requires using fresh shellcheck on CI
        for scr in script_chunk {
            debug!("checking script {}", scr.display());
            cmd.arg(scr);
        }
        cmd.run_check_status();
    }
}

fn task_test() {
    Command::new("cargo").args(&["test"]).run_check_status();
}

fn main() {
    env_logger::init();
    set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();
    let args = CliArgs::from_args();
    match args {
        CliArgs::Check => task_check(),
        CliArgs::Test => task_test(),
    }
    if HAD_ERRORS.load(Ordering::SeqCst) {
        exit(1);
    }
}
