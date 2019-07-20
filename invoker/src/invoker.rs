pub(crate) use crate::invoke_context::InvokeContext;
use crate::{
    compiler::Compiler,
    err,
    inter_api::{
        BuildOutcome, BuildRequest, JudgeRequest, Paths, ValuerNotification, ValuerResponse,
    },
    judge::Judge,
    valuer::Valuer,
    Error,
};
use cfg::Command;
use invoker_api::{status_codes, Status, StatusKind};
use slog::{debug, error};
use snafu::Snafu;
use std::{collections::HashMap, ffi::OsString};

#[derive(Debug, Clone, Snafu)]
pub(crate) enum InterpolateError {
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
pub(crate) fn interpolate_string(
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
    pub(crate) argv: Vec<OsString>,
    pub(crate) env: HashMap<OsString, OsString>,
    pub(crate) cwd: OsString,
}

pub(crate) fn interpolate_command(
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

pub struct Invoker<'a> {
    ctx: InvokeContext<'a>,
}

#[derive(Debug, Clone)]
pub struct InvokeOutcome {
    pub status: Status,
    pub score: u32,
}

impl<'a> Invoker<'a> {
    pub(crate) fn new(ctx: InvokeContext) -> Invoker {
        Invoker { ctx }
    }

    pub(crate) fn invoke(&self) -> Result<InvokeOutcome, Error> {
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
            self.ctx.req.work_dir.path(),
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
        let compiler_request = BuildRequest {
            paths: &build_paths,
        };
        let compiler_response = compiler.compile(compiler_request);
        let artifact = match compiler_response {
            Err(err) => return Err(err),
            Ok(BuildOutcome::Error(st)) => {
                return Ok(InvokeOutcome {
                    status: st,
                    score: 0,
                })
            }
            Ok(BuildOutcome::Success(artifact)) => artifact,
        };

        let mut test_results = vec![];

        let mut valuer = Valuer::new(self.ctx.clone());
        let mut resp = valuer.initial_test();

        let score = loop {
            match resp {
                ValuerResponse::Test { test_id: tid } => {
                    let test = &manifest.tests[(tid - 1) as usize];
                    let run_paths = Paths::new(
                        &self.ctx.req.submission.root_dir,
                        self.ctx.req.work_dir.path(),
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
                    test_results.push((tid, judge_response.clone()));
                    resp = valuer.notify_test_done(ValuerNotification {
                        test_id: tid,
                        test_status: judge_response.status,
                    });
                }
                ValuerResponse::Finish { score } => {
                    break score;
                }
            }
        };

        dbg!(&test_results);

        let status = if score == 100 {
            Status {
                kind: StatusKind::Accepted,
                code: status_codes::ACCEPTED.to_string(),
            }
        } else {
            Status {
                kind: StatusKind::Rejected,
                code: status_codes::PARTIAL_SOLUTION.to_string(),
            }
        };
        debug!(self.ctx.logger, "Invokation finished"; "status" => ?status);
        Ok(InvokeOutcome { status, score })
    }
}
