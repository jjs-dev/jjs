pub(crate) use crate::invoke_context::InvokeContext;
use crate::{
    compiler::Compiler,
    inter_api::{Artifact, BuildOutcome, BuildRequest, JudgeRequest, Paths},
    judge::Judge,
    valuer::Valuer,
    InvokeRequest,
};
use anyhow::{bail, Context};
use cfg::Command;
use invoker_api::{
    status_codes,
    valuer_proto::{TestDoneNotification, ValuerResponse},
    Status, StatusKind,
};
use slog_scope::{debug, warn};
use std::{
    collections::HashMap,
    ffi::OsString,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum InterpolateError {
    #[error("template syntax violation: {message}")]
    BadSyntax { message: &'static str },
    #[error("unknown key {key} in command template")]
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
            return Err(InterpolateError::BadSyntax {
                message: "get pattern start while parsing pattern or pattern end outside of pattern",
            });
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
                    return Err(InterpolateError::MissingKey {
                        key: chunk.to_string(),
                    });
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

struct Notifier {
    score: Option<u32>,
    test: Option<u32>,
    endpoint: Option<String>,
    throttled_until: Instant,
    errored: bool,
}

impl Notifier {
    fn set_score(&mut self, score: u32) {
        self.score = Some(score);
        self.maybe_drain();
    }

    fn set_test(&mut self, test: u32) {
        self.test = Some(test);
        self.maybe_drain();
    }

    fn maybe_drain(&mut self) {
        let mut has_something = false;
        has_something = has_something || self.score.is_some();
        has_something = has_something || self.test.is_some();
        if !has_something {
            return;
        }
        if self.errored {
            return;
        }
        if self.throttled_until > Instant::now() {
            return;
        }
        self.drain();
    }

    fn drain(&mut self) {
        let endpoint = match self.endpoint.as_ref() {
            Some(ep) => ep,
            None => return,
        };
        let event = invoker_api::LiveStatusUpdate {
            score: self.score.take().map(|x| x as i32),
            current_test: self.test.take(),
        };
        let client = reqwest::ClientBuilder::new()
            .timeout(Some(std::time::Duration::from_secs(3)))
            .build()
            .expect("failed to initialize reqwest client");
        debug!("Sending request to {}", &endpoint);
        if let Err(err) = client.post(endpoint).json(&event).send() {
            warn!("Failed to send live status update: {}", err);
            warn!("Disabling live status update for this run");
            self.errored = true;
        }
        self.throttled_until = Instant::now() + LIVE_STATUS_UPDATE_THROTTLE;
    }
}

const LIVE_STATUS_UPDATE_THROTTLE: Duration = Duration::from_nanos(1); //Duration::from_secs(1);

pub struct Invoker<'a> {
    ctx: &'a dyn InvokeContext,
    req: &'a InvokeRequest,
    notifier: Notifier,
}

#[derive(Debug, Clone)]
pub struct InvokeOutcome {
    pub status: Status,
    pub score: u32,
}

impl<'a> Invoker<'a> {
    pub(crate) fn new(ctx: &'a dyn InvokeContext, req: &'a InvokeRequest) -> Invoker<'a> {
        Invoker {
            ctx,
            req,
            notifier: Notifier {
                score: None,
                test: None,
                endpoint: req.live_webhook.clone(),
                throttled_until: Instant::now(),
                errored: false,
            },
        }
    }

    fn run_tests(
        &mut self,
        artifact: &Artifact,
    ) -> anyhow::Result<(InvokeOutcome, invoker_api::valuer_proto::JudgeLog)> {
        let mut test_results = vec![];

        let mut valuer = Valuer::new(self.ctx).context("failed to init valuer")?;
        valuer
            .write_problem_data()
            .context("failed to send problem data")?;

        let (score, treat_as_full, judge_log) = loop {
            match valuer.poll()? {
                ValuerResponse::Test { test_id: tid, live } => {
                    if live {
                        self.notifier.set_test(tid.into());
                    }
                    let tid_u32: u32 = tid.into();
                    let test = &self.ctx.env().problem_data.tests[(tid_u32 - 1u32) as usize];
                    let run_paths = Paths::new(
                        &self.req.run.root_dir,
                        self.req.work_dir.path(),
                        tid.into(),
                        &self.ctx.env().problem_root(),
                    );
                    let judge_request = JudgeRequest {
                        paths: &run_paths,
                        test,
                        test_id: tid.into(),
                        artifact: &artifact,
                    };

                    let judge = Judge {
                        req: judge_request,
                        ctx: self.ctx,
                    };

                    let judge_response = judge
                        .judge()
                        .with_context(|| format!("failed to judge solution on test {}", tid))?;
                    test_results.push((tid, judge_response.clone()));
                    valuer
                        .notify_test_done(TestDoneNotification {
                            test_id: tid,
                            test_status: judge_response.status,
                        })
                        .with_context(|| {
                            format!("failed to notify valuer that test {} is done", tid)
                        })?;
                }
                ValuerResponse::Finish {
                    score,
                    treat_as_full,
                    judge_log,
                } => {
                    break (score, treat_as_full, judge_log);
                }
                ValuerResponse::LiveScore { score } => {
                    self.notifier.set_score(score);
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

    /// Go from valuer judge log to invoker judge log
    fn process_judge_log(
        &self,
        valuer_log: &invoker_api::valuer_proto::JudgeLog,
    ) -> anyhow::Result<crate::judge_log::JudgeLog> {
        use invoker_api::valuer_proto::TestVisibleComponents;
        use std::io::Read;
        let mut persistent_judge_log = crate::judge_log::JudgeLog::default();
        persistent_judge_log.name = valuer_log.name.clone();
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
                let mut stdout_file =
                    std::fs::File::open(stdout_file).context("failed to open output log")?;
                let mut stderr_file =
                    std::fs::File::open(stderr_file).context("failed to open errors log")?;
                stdout_file
                    .read_to_end(&mut compile_stdout)
                    .context("failed to read output log")?;
                stderr_file
                    .read_to_end(&mut compile_stderr)
                    .context("failed to read errors log")?;
            }
            persistent_judge_log.compile_stdout = base64::encode(&compile_stdout);
            persistent_judge_log.compile_stderr = base64::encode(&compile_stderr);
        }
        // for each test, if valuer allowed, add stdin/stdout/stderr etc to judge_log
        {
            for item in &valuer_log.tests {
                let mut new_item = crate::judge_log::JudgeLogTestRow {
                    test_id: item.test_id,
                    test_answer: None,
                    test_stdout: None,
                    test_stderr: None,
                    test_stdin: None,
                    status: None,
                };
                let test_local_dir = self
                    .req
                    .work_dir
                    .path()
                    .join(format!("s-{}", item.test_id.0.get()));
                if item.components.contains(TestVisibleComponents::TEST_DATA) {
                    let test_file = &self.ctx.env().problem_data.tests[item.test_id].path;
                    let test_file = self.ctx.resolve_asset(&test_file);
                    let test_data = std::fs::read(test_file).context("failed to read test data")?;
                    let test_data = base64::encode(&test_data);
                    new_item.test_stdin = Some(test_data);
                }
                if item.components.contains(TestVisibleComponents::OUTPUT) {
                    let stdout_file = test_local_dir.join("stdout.txt");
                    let stderr_file = test_local_dir.join("stderr.txt");
                    //println!("DEBUG: stdout_file={}", stdout_file.display());
                    let sol_stdout =
                        std::fs::read(stdout_file).context("failed to read solution stdout")?;
                    let sol_stderr =
                        std::fs::read(stderr_file).context("failed to read solution stderr")?;
                    let sol_stdout = base64::encode(&sol_stdout);
                    let sol_stderr = base64::encode(&sol_stderr);
                    new_item.test_stdout = Some(sol_stdout);
                    new_item.test_stderr = Some(sol_stderr);
                }
                if item.components.contains(TestVisibleComponents::ANSWER) {
                    let answer_ref = &self.ctx.env().problem_data.tests[item.test_id].correct;
                    if let Some(answer_ref) = answer_ref {
                        let answer_file = self.ctx.resolve_asset(answer_ref);
                        let answer =
                            std::fs::read(answer_file).context("failed to read correct answer")?;
                        let answer = base64::encode(&answer);
                        new_item.test_answer = Some(answer);
                    }
                }
                if item.components.contains(TestVisibleComponents::STATUS) {
                    new_item.status = Some(item.status.clone());
                }
                persistent_judge_log.tests.push(new_item);
            }
        }
        // note that we do not filter subtasks connected staff,
        // because such filtering is done by Valuer.

        Ok(persistent_judge_log)
    }

    pub(crate) fn invoke(mut self) -> anyhow::Result<InvokeOutcome> {
        let compiler = Compiler { ctx: self.ctx };

        let build_paths = Paths::new(
            &self.req.run.root_dir,
            self.req.work_dir.path(),
            0,
            &self.ctx.env().problem_root(),
        );

        if !self.req.run.root_dir.exists() {
            bail!("Submission root dir not exists");
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

        let valuer_judge_log;

        if let Some(art) = artifact {
            let (tests_outcome, jlog) = self.run_tests(&art)?;
            valuer_judge_log = jlog;
            outcome = Some(tests_outcome);
        } else {
            valuer_judge_log = invoker_api::valuer_proto::JudgeLog {
                name: "".to_string(),
                tests: vec![],
                subtasks: vec![],
            };
        }

        let invoker_judge_log = self.process_judge_log(&valuer_judge_log)?;

        let judge_log_path = self.req.work_dir.path().join("log.json");
        debug!("Writing judging log to {}", judge_log_path.display());
        let judge_log_file = std::fs::File::create(&judge_log_path)?;
        let judge_log_file = std::io::BufWriter::new(judge_log_file);
        serde_json::to_writer(judge_log_file, &invoker_judge_log)
            .context("failed to write judge log to file")?;
        let outcome = outcome.unwrap_or_else(|| unreachable!());
        debug!("Invokation finished"; "status" => ?outcome.status);

        Ok(outcome)
    }
}
