use crate::{
    command::Command,
    compile::{
        build::{BuildBackend, Task, TaskError},
        progress_notifier::Notifier,
    },
};
use anyhow::Context as _;
use pom::{FileRef, FileRefRoot, Limits};
use std::{
    collections::HashMap,
    os::unix::{io::IntoRawFd, process::CommandExt},
    path::{Path, PathBuf},
};

/// ProblemBuilder is struct, responsible for building single problem.
/// Its instances are managed by CompilerService.
pub(crate) struct ProblemBuilder<'a> {
    /// Problem manifest
    pub(crate) cfg: &'a crate::manifest::Problem,
    /// Directory, containing problem source files
    pub(crate) problem_dir: &'a Path,
    /// Directory for output files
    pub(crate) out_dir: &'a Path,
    /// Path to local JTL installation
    pub(crate) jtl_dir: &'a Path,
    /// Used to execute build tasks (e.g. builds checker or solution)
    pub(crate) build_backend: &'a dyn BuildBackend,
}

/// Fills given buffer with random hex string
fn get_entropy_hex(buf: &mut [u8]) {
    getrandom::getrandom(buf).expect("get entropy failed");
    for i in buf.iter_mut() {
        *i %= 16;
        if *i < 10 {
            *i += b'0';
        } else {
            *i = b'a' + (*i - 10);
        }
    }
}

/// Applies merge patch `other` to a `place`:
/// If `other` is None, does nothing.
/// If `other` is Some, stores `other` inner value into `place`.
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
    /// Higher-level wrapper for `self.build_backend`
    async fn do_build(&self, src: &Path, dest: &Path) -> anyhow::Result<Command> {
        tokio::fs::create_dir_all(dest)
            .await
            .context("failed to create dir")?;

        let build_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .to_string();
        let build_dir = format!("/tmp/tt-build-{}", &build_id);
        tokio::fs::create_dir(&build_dir)
            .await
            .expect("couldn't create build dir");

        let task = Task {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
            tmp: Path::new(&build_dir).to_path_buf(),
        };
        match self.build_backend.process_task(task.clone()).await {
            Ok(cmd) => Ok(cmd.command),
            Err(err) => {
                eprintln!("Build error: unable to run build task: {}", err);
                if let TaskError::ExitCodeNonZero(out) = err {
                    eprintln!("--- stdout ---\n{}", String::from_utf8_lossy(&out.stdout));
                    eprintln!("--- stderr ---\n{}", String::from_utf8_lossy(&out.stderr));
                }
                eprintln!("Build task: {:#?}", task);
                anyhow::bail!("task execution error")
            }
        }
    }

    /// async wrapper for `glob::glob`
    async fn glob(&self, suffix: &str) -> anyhow::Result<Vec<PathBuf>> {
        let pattern = format!("{}/{}", self.problem_dir.display(), suffix);
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<PathBuf>> {
            let paths = glob::glob(&pattern)
                .context("blob pattern error")?
                .map(|x| match x {
                    Ok(p) => Ok(p),
                    Err(err) => {
                        anyhow::bail!("Glob error: {}", err);
                    }
                })
                .collect::<anyhow::Result<Vec<PathBuf>>>()?;
            Ok(paths)
        })
        .await
        .unwrap()
    }

    /// Builds single solution
    async fn build_solution(&self, sol_path: PathBuf) -> anyhow::Result<(String, Command)> {
        let sol_id = sol_path
            .file_stem()
            .unwrap()
            .to_str()
            .context("path is not utf8")?
            .to_owned();
        println!("Building solution {}", &sol_id);
        let out_path = format!("{}/assets/sol-{}", self.out_dir.display(), &sol_id);
        Ok((
            sol_id,
            self.do_build(&sol_path, &PathBuf::from(&out_path)).await?,
        ))
    }

    /// Builds all solutions
    async fn build_solutions(&self) -> anyhow::Result<HashMap<String, Command>> {
        let mut out = HashMap::new();
        for solution_path in self.glob("solutions/*").await? {
            let (sol_id, cmd) = self.build_solution(solution_path).await?;
            out.insert(sol_id, cmd);
        }
        Ok(out)
    }

    /// Builds single testgen
    async fn build_testgen(
        &self,
        testgen_path: &Path,
        testgen_name: &str,
    ) -> anyhow::Result<Command> {
        println!("Building generator {}", testgen_name);
        let out_path = format!("{}/assets/testgen-{}", self.out_dir.display(), testgen_name);
        self.do_build(testgen_path, &Path::new(&out_path)).await
    }

    /// Builds all testgens
    async fn build_testgens(&self) -> anyhow::Result<HashMap<String, Command>> {
        let mut out = HashMap::new();
        for testgen in self.glob("generators/*").await? {
            let testgen_name = testgen
                .file_stem()
                .unwrap()
                .to_str()
                .context("utf8 error")?;
            let testgen_launch_cmd = self.build_testgen(&testgen, testgen_name).await?;
            out.insert(testgen_name.to_string(), testgen_launch_cmd);
        }
        Ok(out)
    }

    /// Adds common modifications to a child process builder
    fn configure_command(&self, cmd: &mut Command) {
        cmd.current_dir(self.problem_dir);
        cmd.env("JJS_PROBLEM_SRC", &self.problem_dir);
        cmd.env("JJS_PROBLEM_DEST", &self.out_dir);
    }

    /// Builds all tests
    async fn build_tests(
        &self,
        testgens: &HashMap<String, Command>,
        gen_answers: Option<&Command>,
    ) -> anyhow::Result<Vec<pom::Test>> {
        let tests_path = format!("{}/assets/tests", self.out_dir.display());
        std::fs::create_dir_all(&tests_path).expect("couldn't create tests output dir");
        let mut notifier = Notifier::new(self.cfg.tests.len());
        let mut out = vec![];
        for (i, test_spec) in self.cfg.tests.iter().enumerate() {
            let tid = i + 1;
            notifier.maybe_notify(tid);

            let out_file_path = format!("{}/{}-in.txt", &tests_path, tid);
            match &test_spec.gen {
                crate::manifest::TestGenSpec::Generate { testgen, args } => {
                    let testgen_cmd = testgens
                        .get(testgen)
                        .with_context(|| format!("error: unknown testgen {}", testgen))?;

                    let mut entropy_buf = [0; crate::manifest::RANDOM_SEED_LENGTH];
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
                let test_data = tokio::fs::File::open(&out_file_path).await?;

                let correct_file_path = format!("{}/{}-out.txt", &tests_path, tid);

                let answer_data = tokio::fs::File::create(&correct_file_path).await?;

                let mut cmd = cmd.clone();
                self.configure_command(&mut cmd);
                let mut cmd = cmd.to_std_command();
                let mut close_handles = vec![];
                unsafe {
                    let test_data_fd = test_data.into_std().await.into_raw_fd();
                    close_handles.push(test_data_fd);
                    let test_data_fd = libc::dup(test_data_fd);
                    close_handles.push(test_data_fd);

                    let ans_data_fd = answer_data.into_std().await.into_raw_fd();
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
                        "solution stderr: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    anyhow::bail!(
                        "Error while generating correct answer for test {}: main solution failed",
                        tid
                    );
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
        Ok(out)
    }

    /// Builds all checkers (currently only one is supported)
    async fn build_checkers(&self) -> anyhow::Result<FileRef> {
        // TODO: support multi-file checkers
        let checker_path = format!("{}/checkers/main.cpp", self.problem_dir.display());
        self.build_checker(&checker_path).await
    }

    /// Builds single checker
    async fn build_checker(&self, checker_path: &str) -> anyhow::Result<FileRef> {
        let out_path = self.out_dir.join("assets/checker");
        match &self.cfg.check {
            crate::manifest::Check::Custom(_) => {
                self.do_build(Path::new(checker_path), Path::new(&out_path))
                    .await?;
                Ok(FileRef {
                    path: "checker/bin".to_string(),
                    root: FileRefRoot::Problem,
                })
            }
            crate::manifest::Check::Builtin(bc) => {
                let src_path = self
                    .jtl_dir
                    .join(format!("bin/builtin-checker-{}", bc.name));
                tokio::fs::copy(&src_path, &out_path)
                    .await
                    .context("failed to copy checker binary")?;
                Ok(FileRef {
                    path: "checker/bin".to_string(),
                    root: FileRefRoot::Problem,
                })
            }
        }
    }

    /// Builds all modules
    ///
    /// Module is user-defined program. PPC only builds module and places
    /// binaries into compiled problem assets.
    async fn build_modules(&self) -> anyhow::Result<()> {
        for module in self.glob("modules/*").await? {
            let module_name = module.file_name().unwrap().to_str().expect("utf8 error");
            let output_path = self
                .out_dir
                .join("assets")
                .join(format!("module-{}", module_name));
            self.do_build(&module, Path::new(&output_path)).await?;
        }
        Ok(())
    }

    /// Copies files that should just be copied as is.
    /// Currently, only such file is valuer config
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

    /// Main method, which actually builds the problem into
    /// redistributable package.
    pub async fn build(&self) -> anyhow::Result<()> {
        self.build_modules().await?;
        let solutions = self.build_solutions().await?;
        let testgen_launch_info = self.build_testgens().await?;

        let checker_ref = self
            .build_checkers()
            .await
            .context("failed to build checker")?;

        let checker_cmd = self.cfg.check_options.args.clone();

        let tests = {
            let gen_answers = match &self.cfg.check {
                crate::manifest::Check::Custom(cs) => cs.pass_correct,
                crate::manifest::Check::Builtin(_) => true,
            };
            let gen_answers = if gen_answers {
                let primary_solution_name = self.cfg.primary_solution.as_ref().context(
                    "primary-solution must be specified in order to generate tests correct answers",
                )?;
                let sol_data = match solutions.get(primary_solution_name.as_str()) {
                    Some(d) => d,
                    None => {
                        eprint!("Following solutions are defined: ");
                        for sol_name in solutions.keys() {
                            eprint!("{} ", sol_name);
                        }
                        anyhow::bail!("Unknown solution {}", primary_solution_name)
                    }
                };
                Some(sol_data)
            } else {
                None
            };
            self.build_tests(&testgen_launch_info, gen_answers).await?
        };
        if let Err(e) = self.copy_raw() {
            eprintln!("Error: {}", e);
        }

        let valuer_exe = {
            let src = self.jtl_dir.join("bin/jjs-svaluer");
            let dest = self.out_dir.join("assets/valuer");
            tokio::fs::copy(&src, &dest)
                .await
                .context("failed to copy valuer binary")?;
            FileRef {
                root: FileRefRoot::Problem,
                path: "valuer".to_string(),
            }
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
        let manifest_data =
            serde_json::to_string(&problem).context("couldn't serialize manifest")?;
        std::fs::write(manifest_path, manifest_data).context("couldn't emit manifest")
    }
}
