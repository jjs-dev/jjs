mod checker_proto;
mod os_util;

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
    pub problem: &'a pom::Problem,
}

enum WorkItemExtra<'a> {
    Build,
    Run {
        input: &'a [u8],
        checker: String,
        test_cfg: &'a pom::Test,
    },
}

#[derive(Debug, Clone)]
struct Paths {
    problem: String,
    submission: String,
    judge: String,
    step: String,
}

impl Paths {
    fn new(submission_root: &str, judging_id: u32, step_id: u32, problem: &str) -> Paths {
        let submission = submission_root.to_string();
        let judge = format!("{}/j-{}", &submission, judging_id);
        let step = format!("{}/s-{}", &judge, step_id);
        Paths {
            submission,
            judge,
            step,
            problem: problem.to_string(),
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
    Error,
    Other(Status),
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
            WorkResult::Error => Status {
                kind: StatusKind::InternalError,
                code: status_codes::JUDGE_FAULT.to_string(),
            },
            WorkResult::Other(s) => s,
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
            WorkResult::Error => Status {
                kind: StatusKind::InternalError,
                code: status_codes::JUDGE_FAULT.to_string(),
            },
            WorkResult::Other(s) => s,
        }
    }
}

impl<'a> Judger<'a> {
    fn get_asset_path(&self, short_path: &str) -> String {
        format!(
            "{}/var/problems/{}/assets/{}",
            &self.cfg.sysroot, &self.problem.name, short_path
        )
    }

    fn do_task(&self, task: WorkItem) -> WorkResult {
        use std::os::unix::prelude::*;
        fs::create_dir(&task.paths.step).expect("couldn't create per-step dir");
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
            )
                .expect("couldn't copy submission binary into chroot");
        }

        for (i, cmd) in task.commands.iter().enumerate() {
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

            let suffix;
            // for build, we want to distinguish output from different commands
            if let WorkItemExtra::Build { .. } = task.extra {
                suffix = format!("-cmd{}", i);
            } else {
                suffix = "".to_string();
            }
            let stdout_file =
                fs::File::create(format!("{}/stdout{}.txt", &task.paths.step, &suffix))
                    .expect("io error");

            let stderr_file =
                fs::File::create(format!("{}/stderr{}.txt", &task.paths.step, &suffix))
                    .expect("io error");

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
            if let WorkItemExtra::Run { input, .. } = task.extra {
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
        } else if let WorkItemExtra::Run {
            input,
            checker,
            test_cfg,
        } = task.extra
        {
            // run checker
            let sol_file_path = format!("{}/stdout.txt", &task.paths.step);
            let sol_file = fs::File::open(sol_file_path).unwrap();
            let sol_handle = os_util::handle_inherit(sol_file.into_raw_fd().into(), true);
            let full_checker_path = format!("{}/assets/{}", &task.paths.problem, checker);
            let mut cmd = std::process::Command::new(full_checker_path);

            let corr_handle;
            if let Some(corr_path) = &test_cfg.correct {
                let full_path = self.get_asset_path(corr_path);
                let data = fs::read(full_path).unwrap();
                corr_handle = os_util::buffer_to_file(&data, "invoker-correct-data");
            } else {
                corr_handle = os_util::buffer_to_file(&[], "invoker-correct-data");
            }

            let test_handle = os_util::buffer_to_file(input, "invoker-test-data");

            cmd.env("JJS_CORR", corr_handle.to_string());
            cmd.env("JJS_SOL", sol_handle.to_string());
            cmd.env("JJS_TEST", test_handle.to_string());

            let (out_judge_side, out_checker_side) = os_util::make_pipe();
            cmd.env("JJS_CHECKER_OUT", out_checker_side.to_string());
            let (_comments_judge_side, comments_checker_side) = os_util::make_pipe();
            cmd.env("JJS_CHECKER_COMMENT", comments_checker_side.to_string());
            let st = cmd.status().map(|st| st.success());
            os_util::close(out_checker_side);
            os_util::close(comments_checker_side);
            let st = st.unwrap_or(false);
            if !st {
                slog::error!(task.logger, "checker failed");
                return WorkResult::Error;
            }
            let checker_out = match String::from_utf8(os_util::handle_read_all(out_judge_side)) {
                Ok(c) => c,
                Err(_) => {
                    slog::error!(task.logger, "checker produced non-utf8 output");
                    return WorkResult::Error;
                }
            };
            let parsed_out = match checker_proto::parse(&checker_out) {
                Ok(o) => o,
                Err(err) => {
                    slog::error!(task.logger, "checker output couldn't be parsed"; "error" => ?err);
                    return WorkResult::Error;
                }
            };
            return match parsed_out.outcome {
                checker_proto::Outcome::Ok => WorkResult::Ok,
                checker_proto::Outcome::BadChecker => WorkResult::Other(Status {
                    kind: StatusKind::InternalError,
                    code: status_codes::JUDGE_FAULT.to_string(),
                }),
                checker_proto::Outcome::PresentationError => WorkResult::Other(Status {
                    kind: StatusKind::Rejected,
                    code: status_codes::PRESENTATION_ERROR.to_string(),
                }),
                checker_proto::Outcome::WrongAnswer => WorkResult::Other(Status {
                    kind: StatusKind::Rejected,
                    code: status_codes::WRONG_ANSWER.to_string(),
                }),
            };
        }
        WorkResult::Ok
    }

    pub(crate) fn judge(self) -> Status {
        let problem_path = format!(
            "{}/var/problems/{}",
            &self.cfg.sysroot, &self.request.problem.name
        );
        let manifest = self.problem;

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

        let build_paths = Paths::new(
            &self.request.submission_root,
            self.request.judging_id,
            0,
            &problem_path,
        );
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
            let run_paths = Paths::new(
                &self.request.submission_root,
                self.request.judging_id,
                tid,
                &problem_path,
            );
            let run_work_item = WorkItem {
                extra: WorkItemExtra::Run {
                    input: &test_data,
                    checker: manifest.checker.clone(),
                    test_cfg: &test,
                },
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
