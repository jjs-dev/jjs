//! This module implements compiling source package into invoker package
use crate::{command::Command, BeatufilStringExt};
use indicatif::ProgressBar;
use pom::{FileRef, FileRefRoot};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::exit,
};

pub struct ProblemBuilder<'a> {
    pub cfg: &'a crate::cfg::Problem,
    pub problem_dir: &'a Path,
    pub out_dir: &'a Path,
    pub jjs_dir: &'a Path,
    pub args: &'a crate::args::CompileArgs,
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

// TODO: remove duplicated code
impl<'a> ProblemBuilder<'a> {
    /// this function is useful to check TUI (progress bars, text style, etc)
    fn take_sleep(&self) {
        if cfg!(debug_assertions) && std::env::var("SLEEP").is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(2000));
        }
    }

    // TODO support not only cmake
    fn call_magicbuild(&self, src: &Path, out: &Path) -> Command {
        fs::create_dir_all(out).expect("coudln't create dir");

        let build_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .to_string();
        let build_dir = format!("/tmp/tt-build-{}", &build_id);
        fs::create_dir(&build_dir).expect("couldn't create build dir");

        let pb = ProgressBar::new(1);
        pb.set_style(crate::get_progress_bar_style());
        {
            pb.set_message(&"CMake: Configure".style_with(&crate::style::in_progress()));
            let mut cmd = Command::new("cmake");
            cmd.current_dir(&build_dir)
                .arg(src.canonicalize().unwrap().to_str().unwrap());
            if self.args.verbose {
                cmd.arg("-DCMAKE_VERBOSE_MAKEFILE=On");
            }
            cmd.run_quiet();
        }
        {
            pb.set_message(&"CMake: Build".style_with(&crate::style::in_progress()));
            let mut cmd = Command::new("cmake");
            cmd.current_dir(&build_dir).arg("--build").arg(".");
            cmd.run_quiet();
        }
        {
            pb.set_message(&"Copy artifacts".style_with(&crate::style::in_progress()));
            let bin_src = format!("{}/Out", &build_dir);
            let bin_dst = format!("{}/bin", out.display());
            fs::copy(&bin_src, &bin_dst)
                .expect("Couldn't copy CMake-compiled artifact to assets dir (hint: does CMakeLists.txt define Out binary target?)");
        }
        pb.finish_and_clear();
        Command::new(&format!("{}/bin", out.display()))
    }

    fn build_solution(&self, sol_path: PathBuf) -> (String, Command) {
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

    fn build_solutions(&self) -> HashMap<String, Command> {
        let mut out = HashMap::new();
        let solutions_glob = format!("{}/solutions/*", self.problem_dir.display());
        let globs: Vec<_> = glob::glob(&solutions_glob)
            .expect("couldn't glob for solutions")
            .collect();
        let pb = ProgressBar::new(globs.len() as u64);
        pb.set_style(crate::get_progress_bar_style());
        pb.set_message(&"Build solutions".style_with(&crate::style::in_progress()));
        self.take_sleep();
        for solution_path in globs {
            let sol_path = solution_path.expect("io error");
            let pb_msg = format!(
                "Build solution {}",
                sol_path.file_name().unwrap().to_str().expect("ut8 error")
            );
            pb.set_message(&pb_msg.style_with(&crate::style::in_progress()));
            let (sol_id, cmd) = self.build_solution(sol_path);
            out.insert(sol_id, cmd);
            pb.inc(1);
        }
        pb.finish_with_message(&"Build solutions".style_with(&crate::style::ready()));
        out
    }

    fn build_testgen(&self, testgen_path: &Path, testgen_name: &str) -> Command {
        self.take_sleep();
        let out_path = format!("{}/assets/testgen-{}", self.out_dir.display(), testgen_name);
        self.call_magicbuild(testgen_path, &PathBuf::from(&out_path))
    }

    fn build_testgens(&self) -> HashMap<String, Command> {
        let mut out = HashMap::new();
        let testgens_glob = format!("{}/testgens/*", self.problem_dir.display());
        let globs: Vec<_> = glob::glob(&testgens_glob)
            .expect("couldn't glob for testgens")
            .collect();
        let pb = ProgressBar::new(globs.len() as u64);
        pb.set_style(crate::get_progress_bar_style());
        pb.set_message(&"Build testgens".style_with(&crate::style::in_progress()));
        self.take_sleep();
        for testgen in globs {
            let testgen = testgen.expect("io error");
            let testgen_name = testgen.file_name().unwrap().to_str().expect("utf8 error");
            let pb_msg = format!("Build testgen {}", testgen_name);
            pb.set_message(&pb_msg.style_with(&crate::style::in_progress()));
            let testgen_launch_cmd = self.build_testgen(&testgen, testgen_name);
            out.insert(testgen_name.to_string(), testgen_launch_cmd);
            pb.inc(1);
        }
        pb.finish_with_message(&"Build testgens".style_with(&crate::style::ready()));
        out
    }

    fn build_tests(
        &self,
        testgens: &HashMap<String, Command>,
        gen_answers: Option<&Command>,
    ) -> Vec<pom::Test> {
        let pb = ProgressBar::new(self.cfg.tests.len() as u64);
        pb.set_style(crate::get_progress_bar_style());
        pb.set_message(&"Generate tests".style_with(&crate::style::in_progress()));
        let tests_path = format!("{}/assets/tests", self.out_dir.display());
        std::fs::create_dir_all(&tests_path).expect("couldn't create tests output dir");
        let mut out = vec![];
        for (i, test_spec) in self.cfg.tests.iter().enumerate() {
            self.take_sleep();
            let tid = i + 1;
            let out_file_path = format!("{}/{}-in.txt", &tests_path, tid);
            match &test_spec.gen {
                crate::cfg::TestGenSpec::Generate { testgen, args } => {
                    let testgen_cmd = testgens.get(testgen).unwrap_or_else(|| {
                        eprintln!("error: unknown testgen {}", testgen);
                        exit(1);
                    });

                    let mut entropy_buf = [0; 64];
                    get_entropy_hex(&mut entropy_buf);
                    let entropy = String::from_utf8(entropy_buf.to_vec()).unwrap(); // only ASCII can be here

                    let mut cmd = testgen_cmd.clone();
                    for a in args {
                        cmd.arg(a);
                    }
                    cmd.env("JJS_TEST_ID", &tid.to_string());
                    let out_file_handle = crate::open_as_handle(&out_file_path)
                        .expect("couldn't create test output file");
                    cmd.env("JJS_TEST", &out_file_handle.to_string());
                    cmd.env("JJS_RANDOM_SEED", &entropy);
                    let pb_msg = format!("Run: {:?}", &cmd);
                    pb.set_message(&pb_msg.style_with(&crate::style::in_progress()));
                    cmd.run_quiet();
                }
                crate::cfg::TestGenSpec::File { path } => {
                    let path = format!("{}/tests/{}", self.problem_dir.display(), path);
                    let pb_msg = format!("Copy: {}", &path);
                    pb.set_message(&pb_msg.style_with(&crate::style::in_progress()));
                    let test_data = std::fs::read(&path).expect("Couldn't read test data file");
                    std::fs::write(&out_file_path, test_data)
                        .expect("Couldn't write test data to target file");
                }
            }
            let mut test_info = pom::Test {
                path: FileRef {
                    path: format!("tests/{}-in.txt", tid),
                    root: FileRefRoot::Problem,
                },
                correct: None,
            };
            if let Some(cmd) = gen_answers {
                let test_data = fs::read(&out_file_path).unwrap();
                let mut sol = cmd
                    .to_std_command()
                    .stdin(crate::Stdio::piped())
                    .stdout(crate::Stdio::piped())
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
                test_info.correct.replace(FileRef {
                    path: short_file_path,
                    root: FileRefRoot::Problem,
                });
            }
            out.push(test_info);
            pb.inc(1);
        }
        pb.finish_with_message(&"Generate tests".style_with(&crate::style::ready()));
        out
    }

    fn build_checkers(&self) -> FileRef {
        let checker_path = format!("{}/checkers/main", self.problem_dir.display());
        let pb = ProgressBar::new(1);
        pb.set_style(crate::get_progress_bar_style());
        pb.set_message(&"Build checker".style_with(&crate::style::in_progress()));
        self.take_sleep();
        self.build_checker(&checker_path)
    }

    fn build_checker(&self, checker_path: &str) -> FileRef {
        self.take_sleep();
        let out_path = format!("{}/assets/checker", self.out_dir.display());

        match self.cfg.check {
            crate::cfg::Check::Custom(_) => {
                self.call_magicbuild(&PathBuf::from(checker_path), &PathBuf::from(out_path));
                FileRef {
                    path: "checker/bin".to_string(),
                    root: FileRefRoot::Problem,
                }
            }
            crate::cfg::Check::Builtin(ref bc) => FileRef {
                path: format!("bin/builtin-checker-{}", bc.name),
                root: FileRefRoot::System,
            },
        }
    }

    fn build_modules(&self) {
        let testgens_glob = format!("{}/modules/*", self.problem_dir.display());
        let globs: Vec<_> = glob::glob(&testgens_glob)
            .expect("couldn't glob for modules")
            .collect();
        let pb = ProgressBar::new(globs.len() as u64);
        pb.set_style(crate::get_progress_bar_style());
        pb.set_message(&"Build modules".style_with(&crate::style::in_progress()));
        self.take_sleep();
        for module in globs {
            let module = module.expect("io error");
            let module_name = module.file_name().unwrap().to_str().expect("utf8 error");
            let pb_msg = format!("Build module {}", module_name);
            pb.set_message(&pb_msg.style_with(&crate::style::in_progress()));
            let output_path = self
                .out_dir
                .join("assets")
                .join(format!("module-{}", module_name));
            self.take_sleep();
            self.call_magicbuild(&module, &PathBuf::from(&output_path));
            pb.inc(1);
        }
        pb.finish_with_message(&"Build modules".style_with(&crate::style::ready()));
    }

    pub fn build(&self) {
        self.build_modules();
        let solutions = self.build_solutions();
        self.take_sleep();
        let testgen_lauch_info = self.build_testgens();
        self.take_sleep();

        let tests = {
            let gen_answers = match &self.cfg.check {
                crate::cfg::Check::Custom(cs) => cs.pass_correct,
                crate::cfg::Check::Builtin(_) => true,
            };
            let gen_answers = if gen_answers {
                let primary_solution_name = self.cfg.primary_solution.as_ref().unwrap_or_else(|| {
                    eprintln!("primary-solution must be specified in order to generate tests correct answers");
                    exit(1);
                });
                Some(solutions.get(primary_solution_name.as_str()).unwrap())
            } else {
                None
            };
            self.build_tests(&testgen_lauch_info, gen_answers)
        };
        let checker_ref = self.build_checkers();

        let checker_cmd = self.cfg.check_options.args.clone();

        let valuer_exe = FileRef {
            root: FileRefRoot::System,
            path: format!("bin/builtin-valuer-{}", &self.cfg.valuer),
        };

        let problem = pom::Problem {
            title: self.cfg.title.clone(),
            name: self.cfg.name.clone(),
            checker_exe: checker_ref,
            checker_cmd,
            valuer_exe,
            tests,
        };
        let manifest_path = format!("{}/manifest.json", self.out_dir.display());
        let manifest_data = serde_json::to_string(&problem).expect("couldn't serialize manifest");
        std::fs::write(manifest_path, manifest_data).expect("couldn't emit manifest")
    }
}
