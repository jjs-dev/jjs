pub(crate) use crate::invoke_context::InvokeContext;
use crate::{
    compiler::Compiler,
    err,
    inter_api::{
        Artifact, BuildOutcome, BuildRequest, JudgeRequest, Paths, ValuerNotification,
        ValuerResponse,
    },
    judge::Judge,
    judge_log::JudgeLog,
    valuer::Valuer,
    Error, InvokeRequest,
};
use cfg::Command;
use invoker_api::{status_codes, Status, StatusKind};
use slog_scope::{debug, error};
use snafu::Snafu;
use std::{collections::HashMap, ffi::OsString, path::PathBuf};

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
    req: &'a InvokeRequest,
}

#[derive(Debug, Clone)]
pub struct InvokeOutcome {
    pub status: Status,
    pub score: u32,
}

impl<'a> Invoker<'a> {
    pub(crate) fn new(ctx: InvokeContext<'a>, req: &'a InvokeRequest) -> Invoker<'a> {
        Invoker { ctx, req }
    }

    fn problem_path(&self) -> PathBuf {
        self.ctx
            .cfg
            .sysroot
            .join("var/problems")
            .join(&self.ctx.problem_cfg.name)
    }

    fn run_tests(&self, artifact: &Artifact) -> Result<(InvokeOutcome, JudgeLog), Error> {
        let mut test_results = vec![];

        let mut valuer = Valuer::new(self.ctx.clone())?;
        let mut resp = valuer.initial_test()?;

        let (score, treat_as_full, judge_log) = loop {
            match resp {
                ValuerResponse::Test { test_id: tid } => {
                    let test = &self.ctx.problem_data.tests[(tid - 1) as usize];
                    let run_paths = Paths::new(
                        &self.req.submission.root_dir,
                        self.req.work_dir.path(),
                        tid,
                        &self.problem_path(),
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
                    })?;
                }
                ValuerResponse::Finish {
                    score,
                    treat_as_full,
                    judge_log,
                } => {
                    break (score, treat_as_full, judge_log);
                }
            }
        };

        let status = if treat_as_full {
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
        let outcome = InvokeOutcome { status, score };
        Ok((outcome, judge_log))
    }

    fn update_judge_log(&self, log: &mut crate::judge_log::JudgeLog) -> Result<(), Error> {
        use crate::judge_log::VisibleComponents;
        use std::io::Read;
        // now fill compile_stdout and compile_stderr in judge_log
        {
            let mut compile_stdout = Vec::new();
            let mut compile_stderr = Vec::new();
            let compile_dir = self.req.work_dir.path().join("s-0");
            for i in 0.. {
                let stdout_file = compile_dir.join(format!("stdout-{}.txt", i));
                let stderr_file = compile_dir.join(format!("stderr-{}.txt", i));
                if !stdout_file.exists() || !stderr_file.exists() {
                    break;
                }
                let mut stdout_file = std::fs::File::open(stdout_file)?;
                let mut stderr_file = std::fs::File::open(stderr_file)?;
                stdout_file.read_to_end(&mut compile_stdout)?;
                stderr_file.read_to_end(&mut compile_stderr)?;
            }
            log.compile_stdout = base64::encode(&compile_stdout);
            log.compile_stderr = base64::encode(&compile_stderr);
        }
        // if valuer allowed, add stdin/stdout/stderr to judge_log
        {
            for item in &mut log.tests {
                let test_local_dir = self
                    .req
                    .work_dir
                    .path()
                    .join(format!("s-{}", item.test_id.0.get()));
                if item.components.contains(VisibleComponents::TEST_DATA) {
                    let test_file = &self.ctx.problem_data.tests[item.test_id].path;
                    let test_file = self.ctx.get_asset_path(&test_file);
                    let test_data = std::fs::read(test_file)?;
                    let test_data = base64::encode(&test_data);
                    item.test_stdin = Some(test_data);
                }
                if item.components.contains(VisibleComponents::OUTPUT) {
                    let stdout_file = test_local_dir.join("stdout.txt");
                    let stderr_file = test_local_dir.join("stderr.txt");
                    //println!("DEBUG: stdout_file={}", stdout_file.display());
                    let sol_stdout = std::fs::read(stdout_file)?;
                    let sol_stderr = std::fs::read(stderr_file)?;
                    let sol_stdout = base64::encode(&sol_stdout);
                    let sol_stderr = base64::encode(&sol_stderr);
                    item.test_stdout = Some(sol_stdout);
                    item.test_stderr = Some(sol_stderr);
                }
                if item.components.contains(VisibleComponents::ANSWER) {
                    let answer_ref = &self.ctx.problem_data.tests[item.test_id].correct;
                    if let Some(answer_ref) = answer_ref {
                        let answer_file = self.ctx.get_asset_path(answer_ref);
                        let answer = std::fs::read(answer_file)?;
                        let answer = base64::encode(&answer);
                        item.test_answer = Some(answer);
                    }
                }
            }
        }

        Ok(())
    }

    pub(crate) fn invoke(&self) -> Result<InvokeOutcome, Error> {
        let compiler = Compiler {
            ctx: self.ctx.clone(),
        };

        let build_paths = Paths::new(
            &self.req.submission.root_dir,
            self.req.work_dir.path(),
            0,
            &self.problem_path(),
        );

        if !self.req.submission.root_dir.exists() {
            error!("Submission root dir not exists"; "submission" => self.req.submission.props.id);
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

        let mut outcome = None;

        let artifact = match compiler_response {
            Err(err) => return Err(err),
            Ok(BuildOutcome::Error(st)) => {
                outcome = Some(InvokeOutcome {
                    status: st,
                    score: 0,
                });
                None
            }
            Ok(BuildOutcome::Success(artifact)) => Some(artifact),
        };

        let judge_log;

        if let Some(art) = artifact {
            let (tests_outcome, jlog) = self.run_tests(&art)?;
            judge_log = jlog;
            outcome = Some(tests_outcome);
        } else {
            judge_log = JudgeLog {
                name: "".to_string(),
                tests: vec![],
                compile_stdout: "".to_string(),
                compile_stderr: "".to_string(),
            };
        }
        let mut judge_log = judge_log;

        self.update_judge_log(&mut judge_log)?;

        let judge_log_path = self.req.work_dir.path().join("log.json");
        debug!("Writing judging log to {}", judge_log_path.display());
        let judge_log_file = std::fs::File::create(&judge_log_path)?;
        let judge_log_file = std::io::BufWriter::new(judge_log_file);
        serde_json::to_writer(judge_log_file, &judge_log)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>)?;
        let outcome = outcome.unwrap_or_else(|| unreachable!());
        debug!("Invokation finished"; "status" => ?outcome.status);

        Ok(outcome)
    }
}
