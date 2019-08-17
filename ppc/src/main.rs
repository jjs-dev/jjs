#![feature(is_sorted)]

use std::{env, path::PathBuf};

mod cfg;
mod command;
mod compile;
mod import;

mod args {
    use std::path::PathBuf;
    use structopt::StructOpt;

    #[derive(StructOpt)]
    pub struct CompileArgs {
        /// Path to problem package root
        #[structopt(long = "pkg", short = "P")]
        pub pkg_path: PathBuf,
        /// Output path
        #[structopt(long = "out", short = "O")]
        pub out_path: PathBuf,
        /// Rewrite dir
        #[structopt(long = "force", short = "F")]
        pub force: bool,
        /// Verbose
        #[structopt(long = "verbose", short = "V")]
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
        #[structopt(long = "force", short = "F")]
        pub force: bool,
    }

    #[derive(StructOpt)]
    pub enum Args {
        #[structopt(name = "compile")]
        Compile(CompileArgs),
        #[structopt(name = "import")]
        Import(ImportArgs),
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

fn get_progress_bar_style() -> indicatif::ProgressStyle {
    let mut st = indicatif::ProgressStyle::default_bar();
    st = st.template("[{elapsed_precise}] {bar:40.green/blue} {pos:>7}/{len:7} {msg}");
    st
}

mod style {
    pub fn in_progress() -> console::Style {
        console::Style::new().blue()
    }

    pub fn ready() -> console::Style {
        console::Style::new().green()
    }
}

trait BeatufilStringExt: Sized {
    fn style_with(self, s: &console::Style) -> String;
}

impl BeatufilStringExt for &str {
    fn style_with(self, s: &console::Style) -> String {
        s.apply_to(self).to_string()
    }
}

impl BeatufilStringExt for String {
    fn style_with(self, s: &console::Style) -> String {
        (self.as_str()).style_with(s)
    }
}

fn open_as_handle(path: &str) -> std::io::Result<i64> {
    use std::os::unix::io::IntoRawFd;
    // note: platform-dependent code
    let file = std::fs::File::create(path)?;
    let fd = file.into_raw_fd();
    let fd_dup = unsafe { libc::dup(fd) }; // to cancel CLOEXEC behavior
    unsafe {
        libc::close(fd);
    }
    Ok(i64::from(fd_dup))
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

    let raw_problem_cfg: cfg::RawProblem =
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
        jjs_dir: &PathBuf::from(&jjs_dir),
        args: &args,
    };
    builder.build();
}

fn main() {
    use structopt::StructOpt;

    let args = Args::from_args();

    match args {
        Args::Compile(compile_args) => compile_problem(compile_args),
        Args::Import(import_args) => import::exec(import_args),
    }
}
