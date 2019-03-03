//! implements very simple logic
//! if submission compiles, it's considered to be Accepted
//! else it gets Compilation Error
use crate::invoker::{Status, StatusKind};
use cfg::*;
use db::schema::Submission;
use execute as minion;
use std::{collections::BTreeMap, fs, time::Duration};

struct BuildResult {
    status: Status,
}

fn get_toolchain<'a>(submission: &Submission, cfg: &'a Config) -> Option<&'a Toolchain> {
    for t in &cfg.toolchains {
        if submission.toolchain == t.name {
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
/// - fo$$$$(SomeKey))
///
/// Few examples of incorrect strings:
/// - $(
fn interpolate_string(
    string: &str,
    dict: &BTreeMap<String, String>,
) -> Result<String, InterpolateError> {
    // simple FSM

    // states
    const STATE_OUTER: u8 = 0;
    const STATE_OPEN_HALF: u8 = 1;
    const STATE_INNER: u8 = 2;
    let mut state = STATE_OUTER;
    let mut buf = String::new();
    let mut res = String::new();
    // main cycle
    for c in string.chars() {
        match state {
            STATE_OUTER => {
                if c == '$' {
                    state = STATE_OPEN_HALF;
                } else {
                    res.push(c);
                }
            }
            STATE_OPEN_HALF => {
                if c == '(' {
                    state = STATE_INNER;
                } else {
                    res.push('$');
                    res.push(c);
                }
            }
            STATE_INNER => {
                if c == ')' {
                    let value = match dict.get(&buf) {
                        Some(s) => s,
                        None => return Err(InterpolateError::MissingKey(buf)),
                    };
                    res.push_str(&value);
                    buf.clear();
                    state = STATE_OUTER;
                } else {
                    buf.push(c);
                }
            }
            _ => unreachable!("bad state"),
        };
    }
    if state != STATE_OUTER {
        return Err(InterpolateError::BadSyntax(string.to_string()));
    }

    Ok(res)
}

#[derive(Default)]
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

fn build(submission: &Submission, cfg: &Config) -> BuildResult {
    let em = minion::setup();
    let child_root = format!("{}/var/submissions/s-{}", cfg.sysroot, submission.id());
    let child_chroot = format!("{}/chroot", &child_root);
    fs::create_dir(&child_chroot).expect("Couldn't create child chroot");
    let child_share = format!("{}/jjs", &child_root);
    let toolchains_dir = format!("{}/opt", &cfg.sysroot);
    fs::create_dir(&child_share).expect("Couldn't create child share");

    fs::copy(
        format!(
            "{}/var/submissions/s-{}/source",
            cfg.sysroot,
            submission.id()
        ),
        format!("{}/source", &child_share),
    )
    .expect("Couldn't copy submission source into chroot");

    let mut exposed_paths = vec![minion::PathExpositionOptions {
        src: child_share,
        dest: "/jjs".to_string(),
        access: minion::DesiredAccess::Full,
    }];
    {
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
    }

    let dmn = em
        .new_dominion(minion::DominionOptions {
            allow_network: false,
            allow_file_io: false,
            max_alive_process_count: 16,
            memory_limit: 256 * (1 << 20),
            exposed_paths,
            isolation_root: child_chroot,
            time_limit: Duration::from_millis(1000),
        })
        .expect("couldn't create dominion");

    let em = minion::setup();

    let toolchain = get_toolchain(&submission, &cfg);

    let toolchain = match toolchain {
        Some(t) => t,
        None => {
            return BuildResult {
                status: Status {
                    kind: StatusKind::CompilationError,
                    code: "UNKNOWN_TOOLCHAIN".to_string(),
                },
            };
        }
    };

    for cmd in &toolchain.build_commands {
        let mut dict = BTreeMap::new();
        dict.insert(
            String::from("System.SourceFilePath"),
            String::from("/jjs/source"),
        );
        dict.insert(
            String::from("System.BinaryFilePath"),
            String::from("/jjs/build"),
        );

        let interp = interpolate_command(cmd, &dict).expect("syntax error in config");
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
                    minion::HandleWrapper::new(1)
                }),
                stderr: minion::OutputSpecification::RawHandle(unsafe {
                    minion::HandleWrapper::new(2)
                }),
            },
            pwd: cmd.cwd.clone(),
        };
        //dbg!(&opts);

        let mut cp = em.spawn(opts).unwrap();
        let wres = cp.wait_for_exit(Duration::from_secs(3)).unwrap();

        match wres {
            minion::WaitOutcome::Timeout => {
                cp.kill().ok(); //.ok() to ignore
                return BuildResult {
                    status: Status {
                        kind: StatusKind::CompilationError,
                        code: "COMPILATION_TIMED_OUT".to_string(),
                    },
                };
            }
            minion::WaitOutcome::AlreadyFinished => unreachable!("not expected other to wait"),
            minion::WaitOutcome::Exited => {
                if cp.get_exit_code().unwrap().unwrap() != 0 {
                    return BuildResult {
                        status: Status {
                            kind: StatusKind::CompilationError,
                            code: "COMPILER_FAILED".to_string(),
                        },
                    };
                }
            }
        };
    }

    BuildResult {
        status: Status {
            kind: StatusKind::NotSet,
            code: "BUILT".to_string(),
        },
    }
}

pub fn judge(submission: &Submission, cfg: &Config) -> crate::invoker::Status {
    build(submission, cfg).status
}
