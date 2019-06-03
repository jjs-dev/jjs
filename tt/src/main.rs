#![feature(is_sorted, option_xor)]
#[macro_use]
extern crate runtime_fmt;

use std::path::PathBuf;

mod cfg;
mod magic_build;

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
        /// Rewrite dir
        #[structopt(long = "force", short = "F")]
        pub force: bool,
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
use indicatif::ProgressBar;
use std::{path::Path, process::exit};

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

struct ProblemBuilder<'a> {
    cfg: &'a cfg::Problem,
    problem_dir: &'a Path,
    out_dir: &'a Path,
}

mod style {
    pub fn in_progress() -> console::Style {
        console::Style::new()
            .blue()
    }

    pub fn ready() -> console::Style {
        console::Style::new()
            .green()
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

impl<'a> ProblemBuilder<'a> {
    /// this function is useful to check TUI (progress bars, text style, etc)
    fn take_sleep(&self) {
        if cfg!(debug_assertions) && std::env::var("SLEEP").is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(2000));
        }
    }

    fn call_magicbuild(&self, src: &Path, out: &Path) -> magic_build::MagicBuildSpec {
        std::fs::create_dir_all(out).expect("coudln't create dir");
        let params = magic_build::MagicBuildParams {
            path: &src,
            out: &out,
        };

        let pb = ProgressBar::new(1);
        pb.set_style(get_progress_bar_style());
        pb.set_message(&"initializing build".style_with(&style::in_progress()));

        let spec = magic_build::magic_build(params).expect("magic-build error");
        pb.set_length(spec.build.len() as u64);
        self.take_sleep();
        for scmd in &spec.build {
            let mut cmd = scmd.to_std_command();
            pb.set_message(&scmd.to_string_pretty());
            self.take_sleep();
            let status = cmd.status().expect("couldn't execute build command");
            if !status.success() {
                eprintln!("command did not terminated successfully");
                exit(1);
            }
            pb.inc(1);
        }
        pb.finish_and_clear();
        spec
    }

    fn build_solution(&self, sol_path: PathBuf) {
        self.take_sleep();
        let sol_id = sol_path
            .file_name()
            .unwrap()
            .to_str()
            .expect("path is not utf8")
            .to_owned();
        let out_path = format!("{}/assets/sol-{}", self.out_dir.display(), &sol_id);
        self.call_magicbuild(&sol_path, &PathBuf::from(&out_path));
    }

    fn build_solutions(&self) {
        let solutions_glob = format!("{}/solutions/*", self.problem_dir.display());
        let globs: Vec<_> = glob::glob(&solutions_glob)
            .expect("couldn't glob for solutions")
            .collect();
        let pb = ProgressBar::new(globs.len() as u64);
        pb.set_style(get_progress_bar_style());
        pb.set_message(&"build solutions".style_with(&style::in_progress()));
        self.take_sleep();
        for solution_path in globs {
            let sol_path = solution_path.expect("io error");
            let pb_msg = format!("Build solution {}", sol_path.file_name().unwrap().to_str().expect("ut8 error"));
            pb.set_message(&pb_msg.style_with(&style::in_progress()));
            self.build_solution(sol_path);
            pb.inc(1);
        }
        pb.finish_with_message(&"Build solutions".style_with(&style::ready()));
    }

    fn build_testgen(&self, testgen_path: &Path, testgen_name: &str) {
        self.take_sleep();
        let out_path = format!("{}/assets/testgen-{}", self.out_dir.display(), testgen_name);
        self.call_magicbuild(testgen_path, &PathBuf::from(&out_path));
    }

    fn build_testgens(&self) {
        let testgens_glob = format!("{}/testgens/*", self.problem_dir.display());
        let globs: Vec<_> = glob::glob(&testgens_glob)
            .expect("couldn't glob for testgens")
            .collect();
        let pb = ProgressBar::new(globs.len() as u64);
        pb.set_style(get_progress_bar_style());
        pb.set_message(&"Build testgens".style_with(&style::in_progress()));
        self.take_sleep();
        for testgen in globs {
            let testgen = testgen.expect("io error");
            let testgen_name = testgen.file_name().unwrap().to_str().expect("utf8 error");
            let pb_msg = format!("Build testgen {}", testgen_name);
            pb.set_message(&pb_msg.style_with(&style::in_progress()));
            self.build_testgen(&testgen, testgen_name);
            pb.inc(1);
        }
        pb.finish_with_message(&"Build testgens".style_with(&style::ready()));
    }

    fn build(&self) {
        self.build_solutions();
        self.take_sleep();
        self.build_testgens();
    }
}


fn main() {
    use structopt::StructOpt;

    let args = Args::from_args();
    if args.force {
        std::fs::remove_dir_all(&args.out_path).expect("couldn't remove");
        std::fs::create_dir(&args.out_path).expect("couldn't recreate")
    } else {
        check_dir(&args.out_path, false /*TODO*/);
    }
    let toplevel_manifest = args.pkg_path.join("problem.toml");
    let toplevel_manifest = std::fs::read_to_string(toplevel_manifest).unwrap();

    let raw_problem_cfg: cfg::RawProblem =
        toml::from_str(&toplevel_manifest).expect("problem.toml parse error");
    //dbg!(&raw_problem_cfg);
    let problem_cfg = raw_problem_cfg.postprocess().unwrap();
    //dbg!(&problem_cfg);
    let builder = ProblemBuilder {
        cfg: &problem_cfg,
        problem_dir: &args.pkg_path,
        out_dir: &args.out_path,
    };
    builder.build();
}
