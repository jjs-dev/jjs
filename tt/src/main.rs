mod cfg;

mod args {
    use structopt::StructOpt;

    #[derive(StructOpt)]
    pub struct Args {
        /// Path to problem package root
        #[structopt(long = "pkg", short = "P")]
        pub pkg_path: std::path::PathBuf,
        /// Output path
        #[structopt(long = "out", short = "O")]
        pub out_path: std::path::PathBuf,
    }
}

mod errors {
    use snafu::Snafu;
    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    pub enum Error {
        ConfigFormat { description: String },
    }
}

use args::Args;
use std::{path::Path, process::exit};

fn check_dir(path: &Path, allow_nonempty: bool) {
    if !path.exists() {
        eprintln!("error: path {} not exists", path.display());
        exit(1);
    }
    if !path.is_dir() {
        eprintln!("error: path {} is not directory", path.display());
        exit(2);
    }
    if !allow_nonempty && path.read_dir().unwrap().next().is_some() {
        eprintln!("error: dir {} is not empty", path.display());
    }
}

fn build_problem(problem: &cfg::Problem, out_dir: &Path) {
    println!("building solutions");
}

fn main() {
    use structopt::StructOpt;

    let args = Args::from_args();
    check_dir(&args.out_path, false /*TODO*/);

    let toplevel_manifest = args.pkg_path.join("problem.toml");
    let toplevel_manifest = std::fs::read_to_string(toplevel_manifest).unwrap();

    let raw_problem_cfg: cfg::RawProblem =
        toml::from_str(&toplevel_manifest).expect("problem.toml parse error");
    let problem_cfg = raw_problem_cfg.postprocess().unwrap();
    build_problem(&problem_cfg, &args.out_path);
}
