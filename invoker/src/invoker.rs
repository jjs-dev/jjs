mod compiler;
mod invoke_context;
mod judge;

use crate::{err, Error};
use cfg::Command;
use compiler::Compiler;
pub use invoke_context::InvokeContext;
use invoker_api::{status_codes, Status, StatusKind};
use judge::Judge;
use slog::{debug, error};
use snafu::Snafu;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Snafu)]
enum InterpolateError {
    #[snafu(display("template syntax violation: {}", message))]
    BadSyntax { message: String },
    #[snafu(display("unknown key {} in command template", key))]
    MissingKey { key: String },
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
fn interpolate_string(
    string: &str,
    dict: &HashMap<String, OsString>,
) -> Result<OsString, InterpolateError> {
    let ak = aho_corasick::AhoCorasick::new_auto_configured(&["$(", ")"]);
    let matches = ak.find_iter(string);
    let mut out = OsString::new();
    let mut cur_pos = 0;
    let mut next_pat_id = 0;
    for m in matches {
        if m.pattern() != next_pat_id {
            return BadSyntax {
                message:
                    "get pattern start while parsing pattern or pattern end outside of pattern"
                        .to_string(),
            }
            .fail();
        }

        let chunk = &string[cur_pos..m.start()];
        cur_pos = m.end();
        if next_pat_id == 0 {
            out.push(&chunk);
        } else {
            match dict.get(chunk) {
                Some(ref val) => {
                    out.push(val);
                }
                None => {
                    return MissingKey { key: chunk }.fail();
                }
            }
        }
        next_pat_id = 1 - next_pat_id;
    }
    let tail = &string[cur_pos..];
    out.push(tail);
    Ok(out)
}

#[derive(Default, Debug)]
pub(crate) struct CommandInterp {
    argv: Vec<OsString>,
    env: HashMap<OsString, OsString>,
    cwd: OsString,
}

fn interpolate_command(
    command: &Command,
    dict: &HashMap<String, OsString>,
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
    res.cwd = interpolate_string(&command.cwd, dict)?;
    Ok(res)
}

const MEGABYTE: u64 = 1 << 20;

#[derive(Debug, Clone)]
pub(crate) struct Paths {
    problem: PathBuf,
    submission: PathBuf,
    judge: PathBuf,
    step: PathBuf,
}

impl Paths {
    /// external directory child will have RW-access to
    fn share_dir(&self) -> PathBuf {
        self.step.join("share")
    }

    /// Root directory for child
    fn chroot_dir(&self) -> PathBuf {
        self.step.join("chroot")
    }
}

impl Paths {
    fn new(submission_root: &Path, judging_id: u32, step_id: u32, problem: &Path) -> Paths {
        let submission = submission_root.to_path_buf();
        let judge = submission.join(&format!("j-{}", judging_id));
        let step = judge.join(&format!("s-{}", step_id));
        Paths {
            submission,
            judge,
            step,
            problem: problem.to_path_buf(),
        }
    }
}

pub(crate) struct BuildRequest<'a> {
    paths: &'a Paths,
}

/// describes successful build outcome
pub(crate) struct Artifact {
    execute_command: cfg::Command,
}

pub(crate) enum BuildOutcome {
    Success(Artifact),
    Error(Status),
}

pub(crate) struct JudgeRequest<'a> {
    paths: &'a Paths,
    test_id: u32,
    test: &'a pom::Test,
    artifact: &'a Artifact,
}

pub(crate) struct JudgeOutcome {
    status: Status,
}

pub struct Invoker<'a> {
    ctx: InvokeContext<'a>,
}

#[derive(Debug, Clone)]
pub struct InvokeOutcome {
    pub status: Status,
}

impl<'a> Invoker<'a> {
    pub fn new(ctx: InvokeContext) -> Invoker {
        Invoker { ctx }
    }

    pub fn invoke(&self) -> Result<InvokeOutcome, Error> {
        let problem_path = self
            .ctx
            .cfg
            .sysroot
            .join("var/problems")
            .join(&self.ctx.req.problem.name);

        let manifest = &self.ctx.req.problem;

        let compiler = Compiler {
            ctx: self.ctx.clone(),
        };
        let build_paths = Paths::new(
            &self.ctx.req.submission.root_dir,
            self.ctx.req.judging_id,
            0,
            &problem_path,
        );

        if !self.ctx.req.submission.root_dir.exists() {
            error!(self.ctx.logger, "Submission root dir not exists"; "submission" => self.ctx.req.submission.id);
            return Err(Error::BadConfig {
                backtrace: Default::default(),
                inner: Box::new(err::StringError(
                    "Submission root dir not exists".to_string(),
                )),
            });
        }

        fs::create_dir(&build_paths.judge).unwrap_or_else(|e| {
            panic!(
                "couldn't create per-invokation dir at {}: {}",
                &build_paths.judge.display(),
                e
            )
        });
        let compiler_request = BuildRequest {
            paths: &build_paths,
        };
        let compiler_response = compiler.compile(compiler_request);
        let artifact = match compiler_response {
            Err(err) => return Err(err),
            Ok(BuildOutcome::Error(st)) => return Ok(InvokeOutcome { status: st }),
            Ok(BuildOutcome::Success(artifact)) => artifact,
        };

        let mut test_results = vec![];
        for (i, test) in manifest.tests.iter().enumerate() {
            let tid = i as u32 + 1;
            let run_paths = Paths::new(
                &self.ctx.req.submission.root_dir,
                self.ctx.req.judging_id,
                tid,
                &problem_path,
            );
            let judge_request = JudgeRequest {
                paths: &run_paths,
                test,
                test_id: tid,
                artifact: &artifact,
            };

            let judge = Judge {
                req: judge_request,
                ctx: self.ctx.clone(),
            };

            let judge_response = judge.judge()?;
            test_results.push(judge_response);
        }

        let all_tests_passed = test_results
            .iter()
            .all(|judge_outcome| judge_outcome.status.kind == StatusKind::Accepted);

        let status = if all_tests_passed {
            Status {
                kind: StatusKind::Accepted,
                code: status_codes::ACCEPTED.to_string(),
            }
        } else {
            Status {
                kind: StatusKind::Rejected,
                code: "PIECE OF SHIT".to_string(),
            }
        };
        debug!(self.ctx.logger, "Invokation finished"; "status" => ?status);
        Ok(InvokeOutcome { status })
    }
}
