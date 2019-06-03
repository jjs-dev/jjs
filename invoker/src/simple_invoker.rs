//! implements very simple logic
//! if submission compiles, it's considered to be Accepted
//! else it gets Compilation Error
use crate::SubmissionInfo;
use cfg::{Command, Config, Toolchain};
use invoker_api::{status_codes, Status, StatusKind};
use minion;
use slog::{debug, Logger};
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

fn build(
    submission: &SubmissionInfo,
    cfg: &Config,
    invokation_id: &str,
    logger: &Logger,
) -> Status {
    let toolchain = get_toolchain_by_name(&submission.toolchain, &cfg);

    let toolchain = match toolchain {
        Some(t) => t,
        None => {
            return Status {
                kind: StatusKind::InternalError,
                code: status_codes::TOOLCHAIN_SEARCH_ERROR.to_string(),
            };
        }
    };

    let em = minion::setup(); // TODO cache
    fs::create_dir(&submission.chroot_dir).expect("Couldn't create child chroot");
    fs::create_dir(&submission.share_dir).expect("Couldn't create child share");

    fs::copy(
        format!("{}/source", &submission.root_dir),
        format!("{}/{}", &submission.share_dir, &toolchain.filename),
    )
    .expect("Couldn't copy submission source into chroot");

    let mut exposed_paths = derive_path_exposition_options(cfg);

    exposed_paths.push(minion::PathExpositionOptions {
        src: submission.share_dir.clone(),
        dest: "/jjs".to_string(),
        access: minion::DesiredAccess::Full,
    });

    let time_limit = Duration::from_millis(cfg.global_limits.time as u64);

    let dmn = em
        .new_dominion(minion::DominionOptions {
            max_alive_process_count: cfg.global_limits.process_count as _,
            memory_limit: (cfg.global_limits.memory * MEGABYTE) as _,
            exposed_paths,
            isolation_root: submission.chroot_dir.clone(),
            time_limit,
        })
        .expect("couldn't create dominion");

    for cmd in &toolchain.build_commands {
        let mut dict = BTreeMap::new();
        dict.insert(
            String::from("System.SourceFilePath"),
            format!("/jjs/{}", &toolchain.filename),
        );
        dict.insert(
            String::from("System.BinaryFilePath"),
            String::from("/jjs/build"),
        );

        let stdout_file = fs::File::create(format!(
            "{}/build-stdout-{}.txt",
            &submission.root_dir, invokation_id
        ))
        .expect("io error");
        let stderr_file = fs::File::create(format!(
            "{}/build-stderr-{}.txt",
            &submission.root_dir, invokation_id
        ))
        .expect("io error");

        let interp = interpolate_command(cmd, &dict).expect("syntax error in config");
        debug!(logger, "executing command"; "command" => ?interp, "phase" => "build");
        let opts = minion::ChildProcessOptions {
            path: interp.argv[0].clone(),
            arguments: interp.argv[1..].to_vec(),
            environment: interp
                .env
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            dominion: dmn.clone(),
            stdio: minion::StdioSpecification {
                stdin: minion::InputSpecification::Empty,
                stdout: minion::OutputSpecification::RawHandle(unsafe {
                    minion::HandleWrapper::from(stdout_file)
                }),
                stderr: minion::OutputSpecification::RawHandle(unsafe {
                    minion::HandleWrapper::from(stderr_file)
                }),
            },
            pwd: cmd.cwd.clone(),
        };

        let mut cp = em.spawn(opts).unwrap();
        let wres = cp.wait_for_exit(time_limit).unwrap();

        match wres {
            minion::WaitOutcome::Timeout => {
                cp.kill().ok(); //.ok() to ignore: kill on best effort basis
                return Status {
                    kind: StatusKind::CompilationError,
                    code: status_codes::COMPILATION_TIMED_OUT.to_string(),
                };
            }
            minion::WaitOutcome::AlreadyFinished => unreachable!("not expected other to wait"),
            minion::WaitOutcome::Exited => {
                if cp.get_exit_code().unwrap().unwrap() != 0 {
                    return Status {
                        kind: StatusKind::CompilationError,
                        code: status_codes::COMPILER_FAILED.to_string(),
                    };
                }
            }
        };
    }

    fs::copy(
        format!("{}/build", &submission.share_dir),
        format!("{}/build", &submission.root_dir),
    )
    .unwrap();

    Status {
        kind: StatusKind::Accepted,
        code: status_codes::BUILT.to_string(),
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
            return Status {
                kind: StatusKind::Rejected,
                code: status_codes::TIME_LIMIT_EXCEEDED.to_string(),
            };
        }
    }
    if cp.get_exit_code().unwrap().unwrap() != 0 {
        return Status {
            kind: StatusKind::Rejected,
            code: status_codes::RUNTIME_ERROR.to_string(),
        };
    }

    Status {
        kind: StatusKind::Accepted,
        code: status_codes::TEST_PASSED.to_string(),
    }
}

pub fn judge(submission: &SubmissionInfo, cfg: &Config, logger: &Logger) -> Status {
    let build_res = build(submission, cfg, "TODO", logger);
    if build_res.kind != StatusKind::Accepted {
        return build_res;
    }
    run_on_test(submission, cfg, "TODO", b"foo", logger)
}
