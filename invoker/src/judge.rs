//! implements very simple logic
//! if submission compiles and passes all tests, it's considered to be Accepted
//! else it gets Compilation Error
use crate::JudgeRequest;
use cfg::{Command, Config, Toolchain};
use invoker_api::{status_codes, Status, StatusKind};
use minion;
use slog::{debug, o, Logger};
use std::{collections::BTreeMap, fs, time::Duration};

fn get_toolchain_by_name<'a>(name: &str, cfg: &'a Config) -> Option<&'a Toolchain> {
    for t in &cfg.toolchains {
        if name == t.name {
            return Some(t);
        }
    }
    None
}

#[derive(Debug, Clone)]
enum InterpolateError {
    BadSyntax(String),
    MissingKey(String),
}

/// Interpolates string by dictionary
///
/// Few examples of correct template strings:
/// - foo
/// - fo$(KeyName)
/// - fo$$$$(SomeKey)
///
/// Few examples of incorrect strings:
/// - $(
/// - $(SomeKey))
///
fn interpolate_string(
    string: &str,
    dict: &BTreeMap<String, String>,
) -> Result<String, InterpolateError> {
    let ak = aho_corasick::AhoCorasick::new_auto_configured(&["$(", ")"]);
    let matches = ak.find_iter(string);
    let mut out = String::new();
    let mut cur_pos = 0;
    let mut next_pat_id = 0;
    for m in matches {
        if m.pattern() != next_pat_id {
            return Err(InterpolateError::BadSyntax(
                "get pattern start while parsing pattern or pattern end outside of pattern"
                    .to_string(),
            ));
        }

        let chunk = &string[cur_pos..m.start()];
        cur_pos = m.end();
        if next_pat_id == 0 {
            out.push_str(&chunk);
        } else {
            match dict.get(chunk) {
                Some(ref val) => {
                    out.push_str(val);
                }
                None => {
                    return Err(InterpolateError::MissingKey(chunk.to_string()));
                }
            }
        }
        next_pat_id = 1 - next_pat_id;
    }
    let tail = &string[cur_pos..];
    out.push_str(tail);
    Ok(out)
}

#[derive(Default, Debug)]
struct CommandInterp {
    argv: Vec<String>,
    env: BTreeMap<String, String>,
}

fn interpolate_command(
    command: &Command,
    dict: &BTreeMap<String, String>,
) -> Result<CommandInterp, InterpolateError> {
    let mut res: CommandInterp = Default::default();
    for arg in &command.argv {
        let interp = interpolate_string(arg, dict)?;
        res.argv.push(interp);
    }
    for (name, val) in &command.env {
        let name = interpolate_string(name, dict)?;
        let val = interpolate_string(val, dict)?;
        res.env.insert(name, val);
    }
    Ok(res)
}

fn derive_path_exposition_options(cfg: &Config) -> Vec<minion::PathExpositionOptions> {
    let mut exposed_paths = vec![];
    let toolchains_dir = format!("{}/opt", &cfg.sysroot);
    let opt_items = fs::read_dir(&toolchains_dir).expect("Couldn't open child chroot");
    for item in opt_items {
        let item = item.expect("Couldn't open child chroot");
        let item_type = item.file_type().expect("Coudln't get stats");
        if !item_type.is_dir() {
            panic!("couldn't link child chroot, because it contains toplevel-item `{:?}`, which is not directory", item.file_name());
        }
        let name = item.file_name().into_string().expect("utf8 error");
        let peo = minion::PathExpositionOptions {
            src: format!("{}/{}", &toolchains_dir, &name),
            dest: format!("/{}", &name),
            access: minion::DesiredAccess::Readonly,
        };
        exposed_paths.push(peo)
    }
    exposed_paths
}

const MEGABYTE: u64 = 1 << 20;

/// Judges one submission
pub struct Judger<'a> {
    pub cfg: &'a Config,
    pub logger: &'a Logger,
    pub request: &'a JudgeRequest,
}

enum WorkItemExtra<'a> {
    Build,
    Run { input: &'a [u8] },
}

#[derive(Debug, Clone)]
struct Paths {
    submission: String,
    judge: String,
    step: String,
}

impl Paths {
    fn new(submission_root: &str, judging_id: u32, step_id: u32) -> Paths {
        let submission = submission_root.to_string();
        let judge = format!("{}/j-{}", &submission, judging_id);
        let step = format!("{}/s-{}", &judge, step_id);
        Paths {
            submission,
            judge,
            step,
        }
    }
}

struct WorkItem<'a> {
    extra: WorkItemExtra<'a>,
    limits: &'a cfg::Limits,
    logger: Logger,
    backend: &'a dyn minion::Backend,
    commands: &'a [Command],
    paths: &'a Paths,
    toolchain: &'a cfg::Toolchain,
}

#[derive(Debug, Clone)]
enum WorkResult {
    Ok,
    Time,
    Runtime,
}

impl WorkResult {
    fn map_build(self) -> Status {
        match self {
            WorkResult::Ok => Status {
                kind: StatusKind::Accepted,
                code: status_codes::BUILT.to_string(),
            },
            WorkResult::Time => Status {
                kind: StatusKind::CompilationError,
                code: status_codes::COMPILATION_TIMED_OUT.to_string(),
            },
            WorkResult::Runtime => Status {
                kind: StatusKind::CompilationError,
                code: status_codes::COMPILER_FAILED.to_string(),
            },
        }
    }

    fn map_run(self) -> Status {
        match self {
            WorkResult::Ok => Status {
                kind: StatusKind::Accepted,
                code: status_codes::TEST_PASSED.to_string(),
            },
            WorkResult::Time => Status {
                kind: StatusKind::Rejected,
                code: status_codes::TIME_LIMIT_EXCEEDED.to_string(),
            },
            WorkResult::Runtime => Status {
                kind: StatusKind::Rejected,
                code: status_codes::RUNTIME_ERROR.to_string(),
            },
        }
    }
}

impl<'a> Judger<'a> {
    fn read_manifest(&self, problem_path: &str) -> pom::Problem {
        let manifest_path = format!("{}/manifest.json", problem_path);
        let manifest = fs::read(&manifest_path)
            .unwrap_or_else(|_| panic!("couldn't read problem manifest at {}", manifest_path));
        serde_json::from_str(&String::from_utf8_lossy(&manifest)).expect("deserialize failed")
    }

    fn do_task(&self, task: WorkItem) -> WorkResult {
        fs::create_dir(&task.paths.step).expect("couldn't create per-step dir");
        //let judge_dir = format!("{}/j-{}", self.cfg)
        let chroot_dir = format!("{}/chroot", task.paths.step);
        let share_dir = format!("{}/share", task.paths.step);
        fs::create_dir(&chroot_dir).expect("couldn't create chroot");
        fs::create_dir(&share_dir).expect("couldn't create share dir");
        let mut exposed_paths = derive_path_exposition_options(self.cfg);
        exposed_paths.push(minion::PathExpositionOptions {
            src: share_dir.clone(),
            dest: "/jjs".to_string(),
            access: minion::DesiredAccess::Full,
        });
        let time_limit = Duration::from_millis(task.limits.time as u64);
        let dmn_opts = minion::DominionOptions {
            max_alive_process_count: task.limits.process_count as _,
            memory_limit: (task.limits.memory * MEGABYTE) as _,
            exposed_paths,
            isolation_root: chroot_dir.clone(),
            time_limit,
        };
        let dominion = task
            .backend
            .new_dominion(dmn_opts)
            .expect("couldn't create sandbox");

        if let WorkItemExtra::Build = task.extra {
            fs::copy(
                format!("{}/source", &task.paths.submission),
                format!("{}/{}", &share_dir, &task.toolchain.filename),
            )
                .expect("couldn't copy submission source into chroot");
        } else {
            fs::copy(
                format!("{}/build", &task.paths.judge),
                format!("{}/build", &share_dir),
            ).expect("couldn't copy submission binary into chroot");
        }

        for cmd in task.commands {
            let mut dict = BTreeMap::new();
            dict.insert(
                String::from("System.SourceFilePath"),
                format!("/jjs/{}", &task.toolchain.filename),
            );
            dict.insert(
                String::from("System.BinaryFilePath"),
                String::from("/jjs/build"),
            );
            let interp = interpolate_command(cmd, &dict).expect("syntax error in config");
            debug!(task.logger, "executing command"; "command" => ?interp);

            let stdout_file =
                fs::File::create(format!("{}/stdout.txt", &task.paths.step)).expect("io error");

            let stderr_file =
                fs::File::create(format!("{}/stderr.txt", &task.paths.step)).expect("io error");

            let opts = minion::ChildProcessOptions {
                path: interp.argv[0].clone(),
                arguments: interp.argv[1..].to_vec(),
                environment: interp
                    .env
                    .iter()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
                dominion: dominion.clone(),
                stdio: minion::StdioSpecification {
                    stdin: minion::InputSpecification::Pipe,
                    stdout: minion::OutputSpecification::RawHandle(unsafe {
                        minion::HandleWrapper::from(stdout_file)
                    }),
                    stderr: minion::OutputSpecification::RawHandle(unsafe {
                        minion::HandleWrapper::from(stderr_file)
                    }),
                },
                pwd: cmd.cwd.clone(),
            };

            let mut cp = task.backend.spawn(opts).expect("couldn't spawn submission");
            if let WorkItemExtra::Run { input } = task.extra {
                let mut stdin = cp.stdin().unwrap();
                stdin.write_all(input).ok();
            }
            let wres = cp.wait_for_exit(time_limit).unwrap();

            match wres {
                minion::WaitOutcome::Timeout => {
                    cp.kill().ok(); //.ok() to ignore: kill on best effort basis
                    return WorkResult::Time;
                }
                minion::WaitOutcome::AlreadyFinished => unreachable!("not expected other to wait"),
                minion::WaitOutcome::Exited => {
                    if cp.get_exit_code().unwrap().unwrap() != 0 {
                        return WorkResult::Runtime;
                    }
                }
            };
        }
        if let WorkItemExtra::Build = task.extra {
            fs::copy(
                format!("{}/build", &share_dir),
                format!("{}/build", &task.paths.judge),
            )
                .unwrap();
        }
        WorkResult::Ok
    }

    pub(crate) fn judge(self) -> Status {
        let problem_path = format!(
            "{}/var/problems/{}",
            &self.cfg.sysroot, &self.request.problem_name
        );
        let manifest = self.read_manifest(&problem_path);

        //TODO cache
        let backend = minion::setup();

        let toolchain = get_toolchain_by_name(&self.request.submission_props.toolchain, &self.cfg);

        let toolchain = match toolchain {
            Some(t) => t,
            None => {
                return Status {
                    kind: StatusKind::InternalError,
                    code: status_codes::TOOLCHAIN_SEARCH_ERROR.to_string(),
                };
            }
        };

        let build_paths = Paths::new(&self.request.submission_root, /*TODO*/ 1, 0);
        dbg!(&build_paths);
        fs::create_dir(&build_paths.judge).expect("couldn't create per-judge dir");

        let build_work_item = WorkItem {
            extra: WorkItemExtra::Build,
            limits: &self.cfg.global_limits,
            logger: self.logger.new(o!("phase" => "build")),
            backend: &*backend,
            commands: &toolchain.build_commands,
            toolchain: &toolchain,
            paths: &build_paths,
        };
        let build_result = self.do_task(build_work_item).map_build();
        if build_result.kind != StatusKind::Accepted {
            return build_result;
        }

        for (i, test) in manifest.tests.iter().enumerate() {
            let tid = i as u32 + 1;
            let input_file = format!("{}/assets/{}", &problem_path, &test.path);
            let test_data = std::fs::read(input_file).expect("couldn't read test");
            let run_commands = [toolchain.run_command.clone()];
            let run_paths = Paths::new(&self.request.submission_root, /*TODO*/ 1, tid);
            let run_work_item = WorkItem {
                extra: WorkItemExtra::Run { input: &test_data },
                limits: &self.cfg.global_limits,
                logger: self.logger.new(o!("phase" => "run", "test-id" => tid)),
                backend: &*backend,
                commands: &run_commands,
                paths: &run_paths,
                toolchain: &toolchain,
            };
            let run_result = self.do_task(run_work_item).map_run();
            if run_result.kind != StatusKind::Accepted {
                return run_result;
            }
        }

        Status {
            kind: StatusKind::Accepted,
            code: status_codes::ACCEPTED.to_string(),
        }
    }
}

/*
fn build(
    submission: &SubmissionInfo,
    cfg: &Config,
    invokation_id: &str,
    logger: &Logger,
) -> Status {


    let em = minion::setup(); // TODO cache
    fs::create_dir(&submission.chroot_dir).expect("Couldn't create child chroot");
    fs::create_dir(&submission.share_dir).expect("Couldn't create child share");



    let mut exposed_paths = derive_path_exposition_options(cfg);

    exposed_paths.push(minion::PathExpositionOptions {
        src: submission.share_dir.clone(),
        dest: "/jjs".to_string(),
        access: minion::DesiredAccess::Full,
    });

    let time_limit = ;

    let dmn = em)
        .expect("couldn't create dominion");

    for cmd in &toolchain.build_commands {
        let mut dict = BTreeMap::new();




        let interp = interpolate_command(cmd, &dict).expect("syntax error in config");
        debug!(logger, "executing command"; "command" => ?interp, "phase" => "build");
        let opts = minion::ChildProcessOptions {
            path: interp.argv[0].clone(),
            arguments: interp.argv[1..].to_vec(),

        };

        let mut cp = em.spawn(opts).unwrap();

    }




}

pub fn run_on_test(
    submission: &SubmissionInfo,
    cfg: &Config,
    invokation_id: &str,
    test_data: &[u8],
    logger: &Logger,
) -> Status {
    let backend = minion::setup();
    let time_limit = Duration::from_millis(cfg.global_limits.time as _);
    let mut exposed_paths = derive_path_exposition_options(cfg);
    exposed_paths.push(minion::PathExpositionOptions {
        src: submission.share_dir.clone(),
        dest: "/jjs".to_string(),
        access: minion::DesiredAccess::Full,
    });
    let dominion = backend
        .new_dominion(minion::DominionOptions {
            max_alive_process_count: cfg.global_limits.process_count as _,
            memory_limit: cfg.global_limits.memory * MEGABYTE as u64,
            time_limit,
            isolation_root: submission.chroot_dir.clone(),
            exposed_paths,
        })
        .unwrap();
    let mut dict = BTreeMap::new();
    dict.insert(
        String::from("System.BinaryFilePath"),
        String::from("/jjs/build"),
    );
    let toolchain = get_toolchain_by_name(&submission.toolchain, cfg);
    let toolchain = match toolchain {
        Some(t) => t,
        None => {
            return Status {
                kind: StatusKind::InternalError,
                code: status_codes::TOOLCHAIN_SEARCH_ERROR.to_string(),
            };
        }
    };
    let cmd = interpolate_command(&toolchain.run_command, &dict).expect("ill-formed interpolation");
    debug!(logger, "executing command"; "command" => ?cmd, "phase" => "exec");
    let stdout_file = fs::File::create(format!(
        "{}/run-stdout-{}.txt",
        &submission.root_dir, invokation_id
    ))
    .expect("io error");
    let stderr_file = fs::File::create(format!(
        "{}/run-stderr-{}.txt",
        &submission.root_dir, invokation_id
    ))
    .expect("io error");
    let mut cp = backend
        .spawn(minion::ChildProcessOptions {
            path: cmd.argv[0].clone(),
            arguments: cmd.argv[1..].to_vec(),
            environment: cmd.env.into_iter().collect(),
            dominion,
            stdio: unsafe {
                minion::StdioSpecification {
                    stdin: minion::InputSpecification::Pipe,
                    stdout: minion::OutputSpecification::RawHandle(minion::HandleWrapper::from(
                        stdout_file,
                    )),
                    stderr: minion::OutputSpecification::RawHandle(minion::HandleWrapper::from(
                        stderr_file,
                    )),
                }
            },
            pwd: toolchain.run_command.cwd.clone(),
        })
        .expect("Couldn't spawn submission");

    let mut stdin = cp.stdio().0.unwrap();
    stdin.write_all(test_data).ok(); // submission can fail with error, or close it's stdin handle, and so on, so we ignore possible error

    match cp.wait_for_exit(time_limit).expect("couldn't wait") {
        minion::WaitOutcome::AlreadyFinished => unreachable!("mustn't be waited by others"),
        minion::WaitOutcome::Exited => (),
        minion::WaitOutcome::Timeout => {

        }
    }
    if cp.get_exit_code().unwrap().unwrap() != 0 {
        return ;
    }


}

pub fn judge(request: crate::JudgeRequest, cfg: &Config, logger: &Logger) -> Status {
    let problem_path = format!("{}/var/problems/{}", cfg.sysroot, &request.problem_name);
    let manifest_path = format!("{}/manifest.json", &problem_path);
    let manifest = fs::read(&manifest_path)
        .unwrap_or_else(|_| panic!("couldn't read problem manifest at {}", manifest_path));
    let manifest: pom::Problem =
        serde_json::from_str(&String::from_utf8_lossy(&manifest)).expect("deserialize failed");

    let build_res = build(&request.submission, cfg, "TODO", logger);
    if build_res.kind != StatusKind::Accepted {
        return build_res;
    }
    for (i, test) in manifest.tests.iter().enumerate() {

        let res = run_on_test(
            &request.submission,
            cfg,
            &format!("TODO-{}", i + 1),
            &test_data,
            logger,
        );
        if res.kind != StatusKind::Accepted {
            return res;
        }
    }
    Status {
        kind: StatusKind::Accepted,
        code: status_codes::ACCEPTED.to_string(),
    }
}
*/
