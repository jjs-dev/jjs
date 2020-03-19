#![feature(is_sorted)]

mod command;
mod compile;
mod import;
mod manifest;

use std::env;

mod args {
    use std::path::PathBuf;
    use structopt::StructOpt;

    #[derive(Debug, StructOpt)]
    pub struct CompileArgs {
        /// Path to problem package root
        #[structopt(long = "pkg", short = "P")]
        pub pkg_path: PathBuf,
        /// Output path
        #[structopt(long = "out", short = "O")]
        pub out_path: PathBuf,
        /// Rewrite dir
        #[structopt(long, short = "F")]
        pub force: bool,
        /// Verbose
        #[structopt(long, short = "V")]
        pub verbose: bool,
    }

    #[derive(StructOpt)]
    pub struct ImportArgs {
        /// Path to package being imported
        #[structopt(long = "in", short = "I")]
        pub in_path: String,
        /// Out path (will contain ppc package)
        #[structopt(long = "out", short = "O")]
        pub out_path: String,
        /// Rewrite dir
        #[structopt(long, short = "F")]
        pub force: bool,
        /// Write contest config to jjs data_dir.
        /// This option can only be used when importing contest
        #[structopt(long, short = "C")]
        pub update_cfg: bool,
        /// Imported contest name
        /// This option can only be used when importing contest
        #[structopt(long, short = "N")]
        pub contest_name: Option<String>,
        /// Build imported problems and install them to jjs data_dir
        #[structopt(long, short = "B")]
        pub build: bool,
    }

    #[derive(StructOpt)]
    #[structopt(author, about)]
    pub enum Args {
        Compile(CompileArgs),
        Import(ImportArgs),
    }
}

use args::Args;
use std::{
    path::Path,
    process::{exit, Stdio},
};

fn check_dir(path: &Path, allow_nonempty: bool) {
    if !path.exists() {
        eprintln!("error: path {} not exists", path.display());
        exit(1);
    }
    if !path.is_dir() {
        eprintln!("error: path {} is not directory", path.display());
        exit(1);
    }
    if !allow_nonempty && path.read_dir().unwrap().next().is_some() {
        eprintln!("error: dir {} is not empty", path.display());
        exit(1);
    }
}

fn compile_problem(args: args::CompileArgs) {
    if args.force {
        //std::fs::remove_dir_all(&args.out_path).expect("couldn't remove");
        std::fs::create_dir_all(&args.out_path).ok();
    } else {
        check_dir(&args.out_path, false /* TODO */);
    }
    let toplevel_manifest = args.pkg_path.join("problem.toml");
    let toplevel_manifest = std::fs::read_to_string(toplevel_manifest).unwrap();

    let raw_problem_cfg: manifest::RawProblem =
        toml::from_str(&toplevel_manifest).expect("problem.toml parse error");
    let (problem_cfg, warnings) = raw_problem_cfg.postprocess().unwrap();

    if !warnings.is_empty() {
        eprintln!("{} warnings", warnings.len());
        for warn in warnings {
            eprintln!("- {}", warn);
        }
    }

    let jjs_dir = env::var("JJS_PATH").expect("JJS_PATH not set");

    let out_dir = args.out_path.canonicalize().expect("resolve out dir");
    let problem_dir = args.pkg_path.canonicalize().expect("resolve problem dir");

    let builder = compile::ProblemBuilder {
        cfg: &problem_cfg,
        problem_dir: &problem_dir,
        out_dir: &out_dir,
        build_backend: &compile::build::Pibs {
            jjs_dir: Path::new(&jjs_dir),
        },
    };
    builder.build();
}

#[cfg(target_os = "linux")]
fn tune_linux() -> anyhow::Result<()> {
    let mut current_limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        if libc::prlimit(0, libc::RLIMIT_STACK, std::ptr::null(), &mut current_limit) != 0 {
            anyhow::bail!("get current RLIMIT_STACK");
        }
    }
    let new_limit = libc::rlimit {
        rlim_cur: current_limit.rlim_max,
        rlim_max: current_limit.rlim_max,
    };
    unsafe {
        if libc::prlimit(0, libc::RLIMIT_STACK, &new_limit, std::ptr::null_mut()) != 0 {
            anyhow::bail!("update RLIMIT_STACK");
        }
    }

    Ok(())
}

fn tune_resourece_limits() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    tune_linux()?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    use structopt::StructOpt;
    tune_resourece_limits()?;
    let args = Args::from_args();

    match args {
        Args::Compile(compile_args) => {
            compile_problem(compile_args);
            Ok(())
        }
        Args::Import(import_args) => import::exec(import_args),
    }
}
