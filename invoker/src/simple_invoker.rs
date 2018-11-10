//! implements very simple logic
//! if submission compiles, it's considered to be Accepted
//! else it gets Compilation Error
use object;
use invoker;
use config::*;
use execute::{self as minion, ExecutionManager, ChildProcess};
use object::{Submission /*SubmissionContent, FileSubmissionContent,*/};
use invoker::{StatusKind, Status};
use std::{
    collections,
    time::Duration,
    //sync::{Arc, Mutex},
};

//use std::path::{Path, PathBuf};
struct BuildResult {
    //submission: Option<Submission>,
    status: Status,
}

fn prepare_options(_cfg: &Config) -> minion::ChildProcessOptions {
    let mut em = minion::setup();
    let dmn = em.new_dominion(minion::DominionOptions {
        allow_network: false,
        allow_file_io: false,
        max_alive_process_count: 16,
        memory_limit: 0,
    });
    minion::ChildProcessOptions {
        path: String::new(),
        arguments: vec![],
        environment: collections::HashMap::new(),
        dominion: dmn,
    }
}

fn get_toolchain<'a>(submission: &object::Submission, cfg: &'a Config) -> Option<&'a Toolchain> {
    for ref t in &cfg.toolchains {
        if submission.toolchain_name == t.name {
            return Some(t);
        }
    };
    None
}

fn build(submission: &Submission, cfg: &Config) -> BuildResult {
    /*let ref file_path = match submission.content {
        SubmissionContent::File(ref file_submission_content) => {
            file_submission_content
        }
    }.path;*/

    let toolchain = get_toolchain(&submission, &cfg);

    let toolchain = match toolchain {
        Some(t) => t,
        None => {
            return BuildResult {
                //submission: None,
                status: Status {
                    kind: StatusKind::CompilationError,
                    code: "UNKNOWN_TOOLCHAIN".to_string(),
                },
            };
        }
    }.clone();

    for ref cmd in toolchain.build_commands {
        let mut opts = prepare_options(cfg);
        let mut nargs = cmd.argv.clone();
        opts.path = nargs[0].clone();
        opts.arguments = nargs.split_off(1);

        let mut em = minion::setup();

        let mut cp = em.spawn(opts);
        let wres = cp.wait_for_exit(Duration::from_secs(3)).unwrap();

        match wres {
            minion::WaitResult::Timeout => {
                cp.kill();
                return BuildResult {
                    //submission: None,
                    status: Status {
                        kind: StatusKind::CompilationError,
                        code: "COMPILATION_TIMED_OUT".to_string(),
                    },
                };
            }
            minion::WaitResult::AlreadyFinished => panic!("not expected other to wait"),
            minion::WaitResult::Exited => {
                if cp.get_exit_code().unwrap() != 0 {
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
        /*submission: Some(Submission {
            content: SubmissionContent::File(FileSubmissionContent { path: PathBuf::from("/") }),
            toolchain_name: String::new(),
        }),*/
        status: Status { kind: StatusKind::NotSet, code: "BUILT".to_string() },
    }
}

pub fn judge(submission: object::Submission, cfg: &Config) -> invoker::Status {
    build(&submission, cfg).status
}