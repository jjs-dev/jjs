//! implements very simple logic
//! if submission compiles, it's considered to be Accepted
//! else it gets Compilation Error
use crate::invoker::{Status, StatusKind};
use cfg::*;
use db::schema::Submission;
use execute as minion;
use std::{collections, fs, time::Duration};

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

fn build(submission: &Submission, cfg: &Config) -> BuildResult {
    let em = minion::setup();
    let child_root = format!("{}/var/submissions/s-{}", cfg.sysroot, submission.id());
    let child_chroot = format!("{}/chroot", &child_root);
    fs::create_dir(&child_chroot).expect("Couldn't create child chroot");
    let child_share = format!("{}/share", &child_root);
    let toolchains_dir = format!("{}/opt", &cfg.sysroot);
    fs::create_dir(&child_share).expect("Couldn't create child share");
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
        let mut opts = minion::ChildProcessOptions {
            path: String::new(),
            arguments: vec![],
            environment: collections::HashMap::new(),
            dominion: dmn.clone(),
            stdio: minion::StdioSpecification {
                stdin: minion::InputSpecification::Empty,
                stdout: minion::OutputSpecification::Ignore,
                stderr: minion::OutputSpecification::Ignore,
            },
            pwd: "/".to_string(),
        };
        let mut nargs = cmd.argv.clone();
        opts.path = nargs[0].clone();
        opts.arguments = nargs.split_off(1);

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
