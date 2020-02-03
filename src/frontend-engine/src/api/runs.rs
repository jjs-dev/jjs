use super::{
    prelude::*,
    schema::{contest::Problem, RunId},
};
use slog_scope::debug;
use std::path::PathBuf;

pub(crate) struct Run {
    pub id: RunId,
    pub toolchain_name: String,
    pub status: Option<InvokeStatusOut>,
    pub score: Option<i32>,
    pub problem_name: String,
}

impl Run {
    fn data_dir(&self, ctx: &Context) -> PathBuf {
        ctx.cfg
            .sysroot
            .join("var/submissions")
            .join(format!("s-{}", self.id))
    }

    fn last_invoke_dir(&self, ctx: &Context) -> ApiResult<PathBuf> {
        let rejudge_id = self.lookup(ctx)?.rejudge_id;
        let f = format!("i-{}", rejudge_id);
        Ok(self.data_dir(ctx).join(f))
    }

    fn lookup(&self, ctx: &Context) -> ApiResult<db::schema::Run> {
        ctx.db.run_load(self.id).internal(ctx)
    }
}

#[derive(GraphQLInputObject)]
pub(crate) struct InvokeStatusIn {
    pub kind: String,
    pub code: String,
}

#[derive(GraphQLObject)]
pub(crate) struct InvokeStatusOut {
    pub kind: String,
    pub code: String,
}
#[derive(GraphQLInputObject, Copy, Clone)]
pub(crate) struct RunProtocolFilterParams {
    /// If false, compilation logs will be excluded
    compile_log: bool,
    /// If false, test data will be excluded for all tests
    test_data: bool,
    /// If false, solution stdout&stderr will be excluded for all tests
    output: bool,
    /// If false, correct answer will be excluded for all tests
    answer: bool,
}

fn filter_protocol(proto: &mut serde_json::Value, filter: RunProtocolFilterParams) {
    let proto = match proto.as_object_mut() {
        Some(p) => p,
        None => return,
    };
    if !filter.compile_log {
        proto.remove("compile_stdout");
        proto.remove("compile_stderr");
    }
    if let Some(tests) = proto.get_mut("tests") {
        if let Some(tests) = tests.as_array_mut() {
            for test in tests {
                if let Some(test) = test.as_object_mut() {
                    if !filter.output {
                        test.remove("test_stdout");
                        test.remove("test_stderr");
                    }
                    if !filter.test_data {
                        test.remove("test_stdin");
                    }
                    if !filter.answer {
                        test.remove("test_answer");
                    }
                }
            }
        }
    }
}

#[juniper::object(Context = Context)]
impl Run {
    fn id(&self) -> RunId {
        self.id
    }

    fn toolchain(&self, ctx: &Context) -> schema::Toolchain {
        ctx.cfg
            .find_toolchain(&self.toolchain_name)
            .expect("toolchain not found")
            .into()
    }

    fn status(&self) -> &Option<InvokeStatusOut> {
        &self.status
    }

    fn score(&self) -> Option<i32> {
        self.score
    }

    fn problem(&self, ctx: &Context) -> Problem {
        ctx.cfg
            .find_problem(&self.problem_name)
            .expect("problem not found")
            .into()
    }

    /// Returns run source as base64-encoded string
    fn source(&self, ctx: &Context) -> ApiResult<Option<String>> {
        let source_path = self.data_dir(ctx).join("source");
        let source = std::fs::read(source_path).ok();
        let source = source.as_ref().map(base64::encode);
        Ok(source)
    }

    /// Returns run build artifact as base64-encoded string
    fn binary(&self, ctx: &Context) -> ApiResult<Option<String>> {
        let binary_path = self.data_dir(ctx).join("build");
        let binary = std::fs::read(binary_path).ok();
        let binary = binary.as_ref().map(base64::encode);
        Ok(binary)
    }

    /// Returns invocation protocol as JSON string
    fn invocation_protocol(
        &self,
        ctx: &Context,
        filter: RunProtocolFilterParams,
    ) -> ApiResult<Option<String>> {
        let access_ck = ctx.access().wrap_contest("TODO".to_string());
        let kind = access_ck.select_judge_log_kind().internal(ctx)?;
        let path = self.last_invoke_dir(ctx)?.join("log.json");
        debug!("Looking up invocation protocol at {}", path.display());
        let protocol = std::fs::read(path).ok();
        match protocol {
            Some(protocol) => {
                let protocol = String::from_utf8(protocol).internal(ctx)?;
                let protocols: Vec<invoker_api::judge_log::JudgeLog> =
                    serde_json::from_str(&protocol).internal(ctx)?;
                let requested_protocol = protocols.into_iter().find(|p| p.kind == kind);
                let mut requested_protocol = match requested_protocol {
                    Some(rp) => rp,
                    None => return Ok(None),
                };
                let mut requested_protocol =
                    serde_json::to_value(&requested_protocol).internal(ctx)?;
                filter_protocol(&mut requested_protocol, filter);
                let protocol = serde_json::to_string(&requested_protocol).internal(ctx)?;
                Ok(Some(protocol))
            }
            None => Ok(None),
        }
    }

    /// Returns last live status update
    fn live_status_update(&self, ctx: &Context) -> ApiResult<RunLiveStatusUpdate> {
        poll_live_status(ctx, self.id)
    }
}

fn describe_run(ctx: &Context, run: &db::schema::Run) -> ApiResult<Run> {
    let last_inv = ctx.db.inv_last(run.id).internal(ctx)?;
    let kind = ctx
        .access()
        .wrap_contest("TODO".to_string())
        .select_judge_log_kind()
        .internal(ctx)?;
    let inv_out_header = last_inv
        .invoke_outcome_headers()
        .internal(ctx)?
        .into_iter()
        .find(|header| header.kind == kind);
    let status = match inv_out_header.as_ref().and_then(|h| h.status.clone()) {
        Some(s) => Some(InvokeStatusOut {
            kind: s.kind.clone().to_string(),
            code: s.code,
        }),
        None => None,
    };
    Ok(Run {
        id: run.id,
        toolchain_name: run.toolchain_id.clone(),
        status,
        score: inv_out_header.and_then(|h| h.score).map(|sc| sc as i32),
        problem_name: run.problem_id.clone(),
    })
}

pub(super) fn list(ctx: &Context, id: Option<i32>, limit: Option<i32>) -> ApiResult<Vec<Run>> {
    let user_runs = ctx
        .db
        .run_select(id, limit.map(|x| x as u32))
        .internal(ctx)?;
    user_runs.iter().map(|s| describe_run(ctx, s)).collect()
}

pub(super) fn load(ctx: &Context, id: i32) -> ApiResult<Option<Run>> {
    let db_run = ctx.db.run_try_load(id).internal(ctx)?;
    match db_run {
        Some(db_run) => Ok(Some(describe_run(ctx, &db_run)?)),
        None => Ok(None),
    }
}

fn get_lsu_webhook_url(ctx: &Context, run_id: u32) -> Option<String> {
    let live_status_update_key = crate::global::LsuKey {
        user: ctx.token.user_id(),
        run: run_id,
    };

    let lsu_webhook_token = ctx
        .global()
        .live_status_updates
        .make_token(live_status_update_key);

    Some(format!(
        "http://{}:{}/internal/lsu-webhook?token={}",
        ctx.fr_cfg.addr.as_ref()?,
        ctx.fr_cfg.port,
        lsu_webhook_token
    ))
}

pub(super) fn submit_simple(
    ctx: &Context,
    toolchain: schema::ToolchainId,
    code: String,
    problem: schema::ProblemId,
    contest: schema::ContestId,
) -> ApiResult<Run> {
    use db::schema::NewInvocation;
    let toolchain = ctx.cfg.toolchains.iter().find(|t| t.name == toolchain);
    let toolchain = match toolchain {
        Some(tc) => tc.clone(),
        None => return Err(ApiError::new(ctx, "ToolchainUnknown")),
    };
    if contest != "TODO" {
        return Err(ApiError::new(ctx, "ContestUnknown"));
    }
    if !ctx
        .access()
        .wrap_contest(contest)
        .can_submit()
        .internal(ctx)?
    {
        return Err(ApiError::access_denied(ctx));
    }
    let problem = ctx.cfg.contests[0]
        .problems
        .iter()
        .find(|pr| pr.code == problem)
        .cloned();
    let problem = match problem {
        Some(p) => p,
        None => return Err(ApiError::new(ctx, "ProblemUnknown")),
    };
    let prob_name = problem.name;

    let new_run = db::schema::NewRun {
        toolchain_id: toolchain.name,
        problem_id: prob_name,
        rejudge_id: 0,
        user_id: ctx.token.user_id(),
    };

    let run = ctx.db.run_new(new_run).internal(ctx)?;

    // Put run in sysroot
    let run_dir = ctx
        .cfg
        .sysroot
        .join("var/submissions")
        .join(&format!("s-{}", run.id));
    std::fs::create_dir(&run_dir).internal(ctx)?;
    let submission_src_path = run_dir.join("source");
    let decoded_code = base64::decode(&code).report(ctx)?;
    std::fs::write(submission_src_path, &decoded_code).internal(ctx)?;

    // create invocation request
    let invoke_task = invoker_api::DbInvokeTask {
        revision: 0,
        run_id: run.id as u32,
        status_update_callback: get_lsu_webhook_url(ctx, run.id as u32),
    };

    let new_inv = NewInvocation::new(&invoke_task).internal(ctx)?;

    ctx.db.inv_new(new_inv).internal(ctx)?;

    describe_run(ctx, &run)
}

pub(super) fn modify(
    ctx: &Context,
    id: RunId,
    score: Option<i32>,
    rejudge: Option<bool>,
    delete: Option<bool>,
) -> ApiResult<()> {
    if !ctx.access().wrap_run(id).can_modify_run().internal(ctx)? {
        return Err(ApiError::access_denied(ctx));
    }
    let current_run = ctx.db.run_load(id).report(ctx)?;
    let should_delete = delete.unwrap_or(false);
    if should_delete {
        if score.is_some() || rejudge.is_some() {
            return "both modification and delete were requested".report(ctx);
        }
        ctx.db.run_delete(id).internal(ctx)?;
    } else {
        let mut patch = db::schema::RunPatch::default();
        if let Some(true) = rejudge {
            patch.rejudge_id = Some(current_run.rejudge_id + 1);
            // TODO enqueue
        }
        ctx.db.run_update(id, patch).internal(ctx)?;
    }

    Ok(())
}

/// Represents Live Status Updates
///
/// Some fields can be missing for various reasons, it is normal that particular object will look like {liveScore: null, currentTest: null, finish: false}.
/// Information in all fields except `finish` can be inaccurate, incorrect or outdated.
/// You can rely on following: if `finish` is true, final judging results are available
#[derive(GraphQLObject)]
pub(super) struct RunLiveStatusUpdate {
    /// Estimation of score. Usually, final score will be greater than or equal to `live_score`
    live_score: Option<i32>,
    /// Current running test
    current_test: Option<i32>,
    /// Whether final status is available
    finish: bool,
}

pub(super) fn poll_live_status(ctx: &Context, run_id: RunId) -> ApiResult<RunLiveStatusUpdate> {
    let mut lsu = ctx.global();
    let lsu = &mut *lsu;
    let lsu = &mut lsu.live_status_updates;
    let key = crate::global::LsuKey {
        user: ctx.token.user_id(),
        run: run_id as u32,
    };
    let upd = lsu.extract(key);
    debug!("Found update {:?} in cache", &upd);
    if let Some(upd) = upd {
        return Ok(RunLiveStatusUpdate {
            live_score: upd.score,
            current_test: upd.current_test.map(|t| t as i32),
            finish: false,
        });
    }
    let invocation = ctx.db.inv_last(run_id).internal(ctx)?;
    Ok(RunLiveStatusUpdate {
        live_score: None,
        current_test: None,
        finish: invocation.state().internal(ctx)?.is_finished(),
    })
}
