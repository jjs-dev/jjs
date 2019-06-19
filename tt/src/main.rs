#![feature(is_sorted, option_xor)]
#[macro_use]
extern crate runtime_fmt;

use std::{env, fs, path::PathBuf};

mod cfg;
mod command;

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
        /// Verbose
        #[structopt(long = "verbose", short = "V")]
        pub verbose: bool,
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
use std::{
    collections::HashMap,
    io::Write,
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

struct ProblemBuilder<'a> {
    cfg: &'a cfg::Problem,
    problem_dir: &'a Path,
    out_dir: &'a Path,
    jjs_dir: &'a Path,
    args: &'a Args,
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

fn get_entropy_hex(s: &mut [u8]) {
    let n = s.len();
    assert_eq!(n % 2, 0);
    let m = n / 2;
    let rnd_buf = &mut s[m..];
    getrandom::getrandom(rnd_buf).expect("get entropy failed");
    for i in 0..m {
        let c = s[m + i];
        let lo = c % 16;
        let hi = c / 16;
        s[i] = hi;
        s[i + m] = lo;
    }
    for x in s.iter_mut() {
        if *x < 10 {
            *x += b'0';
        } else {
            *x += b'a' - 10;
        }
    }
}

impl<'a> ProblemBuilder<'a> {
    /// this function is useful to check TUI (progress bars, text style, etc)
    fn take_sleep(&self) {
        if cfg!(debug_assertions) && std::env::var("SLEEP").is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(2000));
        }
    }

    // TODO support not only cmake
    fn call_magicbuild(&self, src: &Path, out: &Path) -> command::Command {
        fs::create_dir_all(out).expect("coudln't create dir");

        let build_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .to_string();
        let build_dir = format!("/tmp/tt-build-{}", &build_id);
        fs::create_dir(&build_dir).expect("couldn't create build dir");

        let pb = ProgressBar::new(1);
        pb.set_style(get_progress_bar_style());
        {
            pb.set_message(&"CMake: Configure".style_with(&style::in_progress()));
            let mut cmd = command::Command::new("cmake");
            cmd.current_dir(&build_dir)
                .arg(src.canonicalize().unwrap().to_str().unwrap());
            if self.args.verbose {
                cmd.arg("-DCMAKE_VERBOSE_MAKEFILE=On");
            }
            cmd.run_quiet();
        }
        {
            pb.set_message(&"CMake: Build".style_with(&style::in_progress()));
            let mut cmd = command::Command::new("cmake");
            cmd.current_dir(&build_dir).arg("--build").arg(".");
            cmd.run_quiet();
        }
        {
            pb.set_message(&"Copy artifacts".style_with(&style::in_progress()));
            let bin_src = format!("{}/Out", &build_dir);
            let bin_dst = format!("{}/bin", out.display());
            fs::copy(&bin_src, &bin_dst)
                .expect("Couldn't copy CMake-compiled artifact to assets dir (hint: does CMakeLists.txt define Out binary target?)");
        }
        pb.finish_and_clear();
        command::Command::new(&format!("{}/bin", out.display()))
    }

    fn build_solution(&self, sol_path: PathBuf) -> (String, command::Command) {
        self.take_sleep();
        let sol_id = sol_path
            .file_name()
            .unwrap()
            .to_str()
            .expect("path is not utf8")
            .to_owned();
        let out_path = format!("{}/assets/sol-{}", self.out_dir.display(), &sol_id);
        (
            sol_id,
            self.call_magicbuild(&sol_path, &PathBuf::from(&out_path)),
        )
    }

    fn build_solutions(&self) -> HashMap<String, command::Command> {
        let mut out = HashMap::new();
        let solutions_glob = format!("{}/solutions/*", self.problem_dir.display());
        let globs: Vec<_> = glob::glob(&solutions_glob)
            .expect("couldn't glob for solutions")
            .collect();
        let pb = ProgressBar::new(globs.len() as u64);
        pb.set_style(get_progress_bar_style());
        pb.set_message(&"Build solutions".style_with(&style::in_progress()));
        self.take_sleep();
        for solution_path in globs {
            let sol_path = solution_path.expect("io error");
            let pb_msg = format!(
                "Build solution {}",
                sol_path.file_name().unwrap().to_str().expect("ut8 error")
            );
            pb.set_message(&pb_msg.style_with(&style::in_progress()));
            let (sol_id, cmd) = self.build_solution(sol_path);
            out.insert(sol_id, cmd);
            pb.inc(1);
        }
        pb.finish_with_message(&"Build solutions".style_with(&style::ready()));
        out
    }

    fn build_testgen(&self, testgen_path: &Path, testgen_name: &str) -> command::Command {
        self.take_sleep();
        let out_path = format!("{}/assets/testgen-{}", self.out_dir.display(), testgen_name);
        self.call_magicbuild(testgen_path, &PathBuf::from(&out_path))
    }

    fn build_testgens(&self) -> HashMap<String, command::Command> {
        let mut out = HashMap::new();
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
            let testgen_launch_cmd = self.build_testgen(&testgen, testgen_name);
            out.insert(testgen_name.to_string(), testgen_launch_cmd);
            pb.inc(1);
        }
        pb.finish_with_message(&"Build testgens".style_with(&style::ready()));
        out
    }

    fn build_tests(
        &self,
        testgens: &HashMap<String, command::Command>,
        gen_answers: Option<&command::Command>,
    ) -> Vec<pom::Test> {
        let pb = ProgressBar::new(self.cfg.tests.len() as u64);
        pb.set_style(get_progress_bar_style());
        pb.set_message(&"Generate tests".style_with(&style::in_progress()));
        let tests_path = format!("{}/assets/tests", self.out_dir.display());
        std::fs::create_dir_all(&tests_path).expect("couldn't create tests output dir");
        let mut out = vec![];
        for (i, test_spec) in self.cfg.tests.iter().enumerate() {
            self.take_sleep();
            let tid = i + 1;
            let out_file_path = format!("{}/{}-in.txt", &tests_path, tid);
            match &test_spec.gen {
                cfg::TestGenSpec::Generate { testgen } => {
                    let testgen_cmd = testgens.get(testgen).unwrap_or_else(|| {
                        eprintln!("error: unknown testgen {}", testgen);
                        exit(1);
                    });

                    let mut entropy_buf = [0; 64];
                    get_entropy_hex(&mut entropy_buf);
                    let entropy = String::from_utf8(entropy_buf.to_vec()).unwrap(); // only ASCII can be here

                    let mut cmd = testgen_cmd.clone();
                    cmd.env("JJS_TEST_ID", &tid.to_string());
                    let out_file_handle =
                        open_as_handle(&out_file_path).expect("couldn't create test output file");
                    cmd.env("JJS_TEST", &out_file_handle.to_string());
                    cmd.env("JJS_RANDOM_SEED", &entropy);
                    let pb_msg = format!("Run: {:?}", &cmd);
                    pb.set_message(&pb_msg.style_with(&style::in_progress()));
                    cmd.run_quiet();
                }
                cfg::TestGenSpec::File { path } => {
                    let path = format!("{}/tests/{}", self.problem_dir.display(), path);
                    let pb_msg = format!("Copy: {}", &path);
                    pb.set_message(&pb_msg.style_with(&style::in_progress()));
                    let test_data = std::fs::read(&path).expect("Couldn't read test data file");
                    std::fs::write(&out_file_path, test_data)
                        .expect("Couldn't write test data to target file");
                }
            }
            let mut test_info = pom::Test {
                path: format!("tests/{}-in.txt", tid),
                correct: None,
            };
            if let Some(cmd) = gen_answers {
                let test_data = fs::read(&out_file_path).unwrap();
                let mut sol = cmd
                    .to_std_command()
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap();
                sol.stdin.take().unwrap().write_all(&test_data).ok();
                let out = sol.wait_with_output().unwrap();
                if !out.status.success() {
                    eprintln!(
                        "Error when generating correct answer for test {}: primary solution failed",
                        tid
                    );
                    exit(1);
                }
                let correct_file_path = format!("{}/{}-out.txt", &tests_path, tid);
                fs::write(correct_file_path, &out.stdout).unwrap();
                let short_file_path = format!("tests/{}-out.txt", tid);
                test_info.correct.replace(short_file_path);
            }
            out.push(test_info);
            pb.inc(1);
        }
        pb.finish_with_message(&"Generate tests".style_with(&style::ready()));
        out
    }

    fn build_checkers(&self) {
        let checker_path = format!("{}/checkers/main", self.problem_dir.display());
        let pb = ProgressBar::new(1);
        pb.set_style(get_progress_bar_style());
        pb.set_message(&"Build checker".style_with(&style::in_progress()));
        self.take_sleep();
        self.build_checker(&checker_path);
    }

    fn build_checker(&self, checker_path: &str) {
        self.take_sleep();
        let out_path = format!("{}/assets/checker", self.out_dir.display());

        match self.cfg.check {
            cfg::Check::Custom(_) => {
                self.call_magicbuild(&PathBuf::from(checker_path), &PathBuf::from(out_path));
            }
            cfg::Check::Builtin(ref bc) => {
                fs::create_dir_all(&out_path).unwrap();
                let full_path = format!("{}/bin/checker-{}", self.jjs_dir.display(), &bc.name);
                let out_path = format!("{}/bin", &out_path);
                fs::copy(&full_path, &out_path).unwrap_or_else(|e| {
                    eprintln!(
                        "couldn't copy builtin checker from {} to {}: {}",
                        &full_path, &out_path, e
                    );
                    exit(1);
                });
            }
        }
    }

    fn build(&self) {
        let solutions = self.build_solutions();
        self.take_sleep();
        let testgen_lauch_info = self.build_testgens();
        self.take_sleep();

        let tests = {
            let gen_answers = match &self.cfg.check {
                cfg::Check::Custom(cs) => cs.pass_correct,
                cfg::Check::Builtin(_) => true,
            };
            let gen_answers = if gen_answers {
                Some(solutions.get(&self.cfg.primary_solution).unwrap())
            } else {
                None
            };
            self.build_tests(&testgen_lauch_info, gen_answers)
        };
        self.build_checkers();

        let problem = pom::Problem {
            title: self.cfg.title.clone(),
            name: self.cfg.name.clone(),
            checker: "checker/bin".to_string(),
            tests,
        };
        let manifest_path = format!("{}/manifest.json", self.out_dir.display());
        let manifest_data = serde_json::to_string(&problem).expect("couldn't serialize manifest");
        std::fs::write(manifest_path, manifest_data).expect("couldn't emit manifest")
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
    let (problem_cfg, warnings) = raw_problem_cfg.postprocess().unwrap();

    if !warnings.is_empty() {
        eprintln!("{} warnings", warnings.len());
        for warn in warnings {
            eprintln!("- {}", warn);
        }
    }

    let jjs_dir = env::var("JJS_PATH").expect("JJS_PATH not set");

    let builder = ProblemBuilder {
        cfg: &problem_cfg,
        problem_dir: &args.pkg_path,
        out_dir: &args.out_path,
        jjs_dir: &PathBuf::from(&jjs_dir),
        args: &args,
    };
    builder.build();
}
