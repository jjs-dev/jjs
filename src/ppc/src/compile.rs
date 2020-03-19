//! This module implements compiling source package into invoker package
pub(crate) mod build;
mod progress_notifier;

use crate::command::Command;
use pom::{FileRef, FileRefRoot, Limits};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::exit,
};

pub(crate) struct ProblemBuilder<'a> {
    pub(crate) cfg: &'a crate::manifest::Problem,
    pub(crate) problem_dir: &'a Path,
    pub(crate) out_dir: &'a Path,
    pub(crate) build_backend: &'a dyn build::BuildBackend,
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

fn merge_option<T: Copy>(place: &mut Option<T>, other: Option<T>) {
    if let Some(x) = other {
        place.replace(x);
    }
}

/// Merges several `Limits` object. Last element of slice will have maximal proirity.
fn merge_limits(limits_set: &[Limits]) -> Limits {
    let mut res = Limits::default();
    for lim in limits_set {
        merge_option(&mut res.memory, lim.memory);
        merge_option(&mut res.process_count, lim.process_count);
        merge_option(&mut res.time, lim.time);
    }
    res
}

// TODO: remove duplicated code
impl<'a> ProblemBuilder<'a> {
    fn do_build(&self, src: &Path, dest: &Path) -> Command {
        fs::create_dir_all(dest).expect("failed to create dir");

        let build_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .to_string();
        let build_dir = format!("/tmp/tt-build-{}", &build_id);
        fs::create_dir(&build_dir).expect("couldn't create build dir");

        let task = build::Task {
            src,
            dest,
            tmp: Path::new(&build_dir),
        };
        match self.build_backend.process_task(task) {
            Ok(cmd) => cmd.command,
            Err(err) => {
                eprintln!("Build error: unable to run build task: {}", err);
                if let build::TaskError::ExitCodeNonZero(out) = err {
                    eprintln!("--- stdout ---\n{}", String::from_utf8_lossy(&out.stdout));
                    eprintln!("--- stderr ---\n{}", String::from_utf8_lossy(&out.stderr));
                }
                eprintln!("Build task: {:#?}", task);
                exit(1);
            }
        }
    }

    fn glob(&self, suffix: &str) -> Vec<PathBuf> {
        let pattern = format!("{}/{}", self.problem_dir.display(), suffix);
        match glob::glob(&pattern) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("Glob error: {}", e);
                exit(1);
            }
        }
        .map(|x| match x {
            Ok(p) => p,
            Err(err) => {
                eprintln!("Glob error: {}", err);
                exit(1);
            }
        })
        .collect()
    }

    fn build_solution(&self, sol_path: PathBuf) -> (String, Command) {
        let sol_id = sol_path
            .file_stem()
            .unwrap()
            .to_str()
            .expect("path is not utf8")
            .to_owned();
        println!("Building solution {}", &sol_id);
        let out_path = format!("{}/assets/sol-{}", self.out_dir.display(), &sol_id);
        (sol_id, self.do_build(&sol_path, &PathBuf::from(&out_path)))
    }

    fn build_solutions(&self) -> HashMap<String, Command> {
        let mut out = HashMap::new();
        for solution_path in self.glob("solutions/*") {
            let (sol_id, cmd) = self.build_solution(solution_path);
            out.insert(sol_id, cmd);
        }
        out
    }

    fn build_testgen(&self, testgen_path: &Path, testgen_name: &str) -> Command {
        println!("Building generator {}", testgen_name);
        let out_path = format!("{}/assets/testgen-{}", self.out_dir.display(), testgen_name);
        self.do_build(testgen_path, &Path::new(&out_path))
    }

    fn build_testgens(&self) -> HashMap<String, Command> {
        let mut out = HashMap::new();
        for testgen in self.glob("generators/*") {
            let testgen_name = testgen.file_stem().unwrap().to_str().expect("utf8 error");
            let testgen_launch_cmd = self.build_testgen(&testgen, testgen_name);
            out.insert(testgen_name.to_string(), testgen_launch_cmd);
        }
        out
    }

    fn configure_command(&self, cmd: &mut Command) {
        cmd.current_dir(self.problem_dir);
        cmd.env("JJS_PROBLEM_SRC", &self.problem_dir);
        cmd.env("JJS_PROBLEM_DEST", &self.out_dir);
    }

    fn build_tests(
        &self,
        testgens: &HashMap<String, Command>,
        gen_answers: Option<&Command>,
    ) -> Vec<pom::Test> {
        let tests_path = format!("{}/assets/tests", self.out_dir.display());
        std::fs::create_dir_all(&tests_path).expect("couldn't create tests output dir");
        let mut notifier = progress_notifier::Notifier::new(self.cfg.tests.len());
        let mut out = vec![];
        for (i, test_spec) in self.cfg.tests.iter().enumerate() {
            let tid = i + 1;
            notifier.maybe_notify(tid);

            let out_file_path = format!("{}/{}-in.txt", &tests_path, tid);
            match &test_spec.gen {
                crate::manifest::TestGenSpec::Generate { testgen, args } => {
                    let testgen_cmd = testgens.get(testgen).unwrap_or_else(|| {
                        eprintln!("error: unknown testgen {}", testgen);
                        exit(1);
                    });

                    let mut entropy_buf = [0; 16];
                    get_entropy_hex(&mut entropy_buf);
                    let entropy = String::from_utf8(entropy_buf.to_vec()).unwrap(); // only ASCII can be here

                    let mut cmd = testgen_cmd.clone();
                    for a in args {
                        cmd.arg(a);
                    }
                    cmd.env("JJS_TEST_ID", &tid.to_string());
                    cmd.env("JJS_RANDOM_SEED", &entropy);
                    self.configure_command(&mut cmd);
                    let gen_out = cmd.run_quiet();
                    std::fs::write(&out_file_path, gen_out.stdout).expect("failed to write test");
                }
                crate::manifest::TestGenSpec::File { path } => {
                    let src_path = self.problem_dir.join("tests").join(path);
                    if let Err(e) = std::fs::copy(&src_path, &out_file_path) {
                        eprintln!(
                            "Couldn't copy test data from {} to {}: {}",
                            src_path.display(),
                            out_file_path,
                            e,
                        );
                    }
                }
            }
            let mut test_info = pom::Test {
                path: FileRef {
                    path: format!("tests/{}-in.txt", tid),
                    root: FileRefRoot::Problem,
                },
                correct: None,
                limits: merge_limits(&[self.cfg.limits, test_spec.limits]),
                group: test_spec.group.clone(),
            };
            if let Some(cmd) = gen_answers {
                let test_data = fs::File::open(&out_file_path).unwrap();

                let correct_file_path = format!("{}/{}-out.txt", &tests_path, tid);

                let answer_data = fs::File::create(&correct_file_path).unwrap();

                let mut cmd = cmd.clone();
                self.configure_command(&mut cmd);
                let mut cmd = cmd.to_std_command();
                let mut close_handles = vec![];
                unsafe {
                    use std::os::unix::{io::IntoRawFd, process::CommandExt};
                    let test_data_fd = test_data.into_raw_fd();
                    close_handles.push(test_data_fd);
                    let test_data_fd = libc::dup(test_data_fd);
                    close_handles.push(test_data_fd);

                    let ans_data_fd = answer_data.into_raw_fd();
                    close_handles.push(ans_data_fd);
                    let ans_data_fd = libc::dup(ans_data_fd);
                    close_handles.push(ans_data_fd);
                    cmd.pre_exec(move || {
                        if libc::dup2(test_data_fd, 0) == -1 {
                            return Err(std::io::Error::last_os_error());
                        }
                        if libc::dup2(ans_data_fd, 1) == -1 {
                            return Err(std::io::Error::last_os_error());
                        }
                        Ok(())
                    });
                }
                let output = cmd
                    .stdin(crate::Stdio::piped())
                    .stdout(crate::Stdio::piped())
                    .stderr(crate::Stdio::piped())
                    .output()
                    .unwrap_or_else(|err| panic!("launch main solution error: {}", err));
                if !output.status.success() {
                    eprintln!(
                        "Error when generating correct answer for test {}: main solution failed",
                        tid
                    );
                    eprintln!(
                        "solution stderr: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    exit(1);
                }
                let short_file_path = format!("tests/{}-out.txt", tid);
                test_info.correct.replace(FileRef {
                    path: short_file_path,
                    root: FileRefRoot::Problem,
                });
                for handle in close_handles {
                    unsafe {
                        libc::close(handle);
                    }
                }
            }
            out.push(test_info);
        }
        out
    }

    fn build_checkers(&self) -> FileRef {
        // TODO: support multi-file checkers
        let checker_path = format!("{}/checkers/main.cpp", self.problem_dir.display());
        self.build_checker(&checker_path)
    }

    fn build_checker(&self, checker_path: &str) -> FileRef {
        let out_path = format!("{}/assets/checker", self.out_dir.display());

        match self.cfg.check {
            crate::manifest::Check::Custom(_) => {
                self.do_build(Path::new(checker_path), Path::new(&out_path));
                FileRef {
                    path: "checker/bin".to_string(),
                    root: FileRefRoot::Problem,
                }
            }
            crate::manifest::Check::Builtin(ref bc) => FileRef {
                path: format!("bin/builtin-checker-{}", bc.name),
                root: FileRefRoot::System,
            },
        }
    }

    fn build_modules(&self) {
        for module in self.glob("modules/*") {
            let module_name = module.file_name().unwrap().to_str().expect("utf8 error");
            let output_path = self
                .out_dir
                .join("assets")
                .join(format!("module-{}", module_name));
            self.do_build(&module, Path::new(&output_path));
        }
    }

    fn copy_raw(&self) -> std::io::Result<()> {
        let valuer_cfg_dir = self.out_dir.join("assets/valuer-cfg");
        if let Some(valuer_cfg) = &self.cfg.valuer_cfg {
            println!("Valuer config");
            let src = self.problem_dir.join(valuer_cfg.trim_start_matches('/'));
            let dest = valuer_cfg_dir.join("cfg.yaml");
            std::fs::create_dir(&valuer_cfg_dir)?;
            if src.is_file() {
                std::fs::copy(&src, &dest)?;
            } else {
                // TODO
                eprintln!("Multi-file valuer config is TODO");
            }
        }
        Ok(())
    }

    pub fn build(&self) {
        self.build_modules();
        let solutions = self.build_solutions();
        let testgen_launch_info = self.build_testgens();

        let checker_ref = self.build_checkers();

        let checker_cmd = self.cfg.check_options.args.clone();

        if let Ok(s) = std::env::var("PPC_DEV_SKIP_TESTS") {
            if !s.is_empty() {
                return;
            }
        }

        let tests = {
            let gen_answers = match &self.cfg.check {
                crate::manifest::Check::Custom(cs) => cs.pass_correct,
                crate::manifest::Check::Builtin(_) => true,
            };
            let gen_answers = if gen_answers {
                let primary_solution_name = self.cfg.primary_solution.as_ref().unwrap_or_else(|| {
                    eprintln!("primary-solution must be specified in order to generate tests correct answers");
                    exit(1);
                });
                let sol_data = match solutions.get(primary_solution_name.as_str()) {
                    Some(d) => d,
                    None => {
                        eprintln!("Unknown solution {}", primary_solution_name);
                        eprint!("Following solutions are defined: ");
                        for sol_name in solutions.keys() {
                            eprint!("{} ", sol_name);
                        }
                        eprintln!();
                        exit(1);
                    }
                };
                Some(sol_data)
            } else {
                None
            };
            self.build_tests(&testgen_launch_info, gen_answers)
        };
        if let Err(e) = self.copy_raw() {
            eprintln!("Error: {}", e);
        }

        let valuer_exe = FileRef {
            root: FileRefRoot::System,
            path: "bin/jjs-svaluer".to_string(),
        };

        let valuer_cfg = FileRef {
            root: FileRefRoot::Problem,
            path: "valuer-cfg".to_string(),
        };

        let problem = pom::Problem {
            title: self.cfg.title.clone(),
            name: self.cfg.name.clone(),
            checker_exe: checker_ref,
            checker_cmd,
            valuer_exe,
            tests,
            valuer_cfg,
        };
        let manifest_path = format!("{}/manifest.json", self.out_dir.display());
        let manifest_data = serde_json::to_string(&problem).expect("couldn't serialize manifest");
        std::fs::write(manifest_path, manifest_data).expect("couldn't emit manifest")
    }
}
