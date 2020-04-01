use super::{
    prelude::*,
    schema::{ContestId, RunId},
};
use futures::stream::{StreamExt, TryStreamExt};
use slog_scope::debug;
use std::path::PathBuf;

/// Represents a run.
#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct Run {
    pub id: RunId,
    pub contest_id: ContestId,
    pub toolchain_name: String,
    pub status: Option<InvokeStatus>,
    pub score: Option<i32>,
    pub problem_name: String,
}

impl ApiObject for Run {
    fn name() -> &'static str {
        "Run"
    }
}

fn run_data_dir(ctx: &Context, id: RunId) -> PathBuf {
    ctx.data_dir.join("var/runs").join(format!("run.{}", id))
}

async fn run_lookup(ctx: &Context, id: RunId) -> ApiResult<db::schema::Run> {
    ctx.db().run_load(id).await.internal(ctx)
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct InvokeStatus {
    pub kind: String,
    pub code: String,
}

impl ApiObject for InvokeStatus {
    fn name() -> &'static str {
        "InvokeStatus"
    }
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

async fn describe_run(ctx: &Context, run: &db::schema::Run) -> ApiResult<Run> {
    let last_inv = ctx.db().inv_last(run.id).await.internal(ctx)?;
    let kind = ctx
        .access()
        .wrap_contest(run.contest_id.clone())
        .select_judge_log_kind()
        .internal(ctx)?;
    let inv_out_header = last_inv
        .invoke_outcome_headers()
        .internal(ctx)?
        .into_iter()
        .find(|header| header.kind == kind);
    let status = match inv_out_header.as_ref().and_then(|h| h.status.clone()) {
        Some(s) => Some(InvokeStatus {
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
        contest_id: run.contest_id.clone(),
    })
}

#[get("/runs?<limit>")]
pub(crate) async fn route_list(ctx: Context, limit: Option<i32>) -> ApiResult<Json<Vec<Run>>> {
    let user_runs = ctx
        .db()
        .run_select(
            None, /* TODO: remove this param */
            limit.map(|x| x as u32),
        )
        .await
        .internal(&ctx)?;
    Ok(Json(
        futures::stream::iter(user_runs.iter())
            .then(|s| describe_run(&ctx, s))
            .try_collect::<Vec<_>>()
            .await?,
    ))
}

#[get("/runs/<id>")]
pub(crate) async fn route_get(ctx: Context, id: i32) -> ApiResult<Json<Option<Run>>> {
    let db_run = ctx.db().run_try_load(id).await.internal(&ctx)?;
    match db_run {
        Some(db_run) => Ok(Json(Some(describe_run(&ctx, &db_run).await?))),
        None => Ok(Json(None)),
    }
}

async fn get_lsu_webhook_url(ctx: &Context, run_id: u32) -> Option<String> {
    let live_status_update_key = crate::global::LsuKey {
        user: ctx.token.user_id(),
        run: run_id,
    };

    let lsu_webhook_token = ctx
        .global().await
        .live_status_updates
        .make_token(live_status_update_key);

    Some(format!(
        "http://{}:{}/internal/lsu-webhook?token={}",
        ctx.config().external_addr.as_ref()?,
        ctx.config().listen.port,
        lsu_webhook_token
    ))
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct RunSimpleSubmitParams {
    /// Toolchain to use when judging this run
    toolchain: schema::ToolchainId,
    ///  Base64-encoded source text
    code: String,
    /// Problem name, relative to contest
    problem: schema::ProblemId,
    /// Contest where run is submitted
    contest: schema::ContestId,
}

impl ApiObject for RunSimpleSubmitParams {
    fn name() -> &'static str {
        "RunSimpleSubmitParams"
    }
}

#[post("/runs", data = "<p>")]
pub(crate) async fn route_submit_simple(
    ctx: Context,
    p: Json<RunSimpleSubmitParams>,
) -> ApiResult<Json<Run>> {
    use db::schema::NewInvocation;
    let toolchain = ctx.cfg.find::<entity::Toolchain>(&p.toolchain);
    let toolchain = match toolchain {
        Some(tc) => tc.clone(),
        None => return Err(ApiError::new(&ctx, "ToolchainUnknown")),
    };
    let contest: &entity::Contest = match ctx.cfg.find(&p.contest) {
        Some(ent) => ent,
        None => return Err(ApiError::new(&ctx, "ContestUnknown")),
    };
    if !ctx
        .access()
        .wrap_contest(contest.id.clone())
        .can_submit()
        .internal(&ctx)?
    {
        return Err(ApiError::access_denied(&ctx));
    }
    let problem = contest
        .problems
        .iter()
        .find(|pr| pr.code == p.problem)
        .cloned();
    let problem = match problem {
        Some(p) => p,
        None => return Err(ApiError::new(&ctx, "ProblemUnknown")),
    };
    let prob_name = problem.name;

    let new_run = db::schema::NewRun {
        toolchain_id: toolchain.name,
        problem_id: prob_name,
        rejudge_id: 0,
        user_id: ctx.token.user_id(),
        contest_id: contest.id.to_string(),
    };

    let run = ctx.db().run_new(new_run).await.internal(&ctx)?;

    // Put run in sysroot
    let run_dir = ctx
        .data_dir
        .join("var/runs")
        .join(&format!("run.{}", run.id));
    std::fs::create_dir(&run_dir).internal(&ctx)?;
    let submission_src_path = run_dir.join("source");
    let decoded_code = base64::decode(&p.code).report(&ctx)?;
    std::fs::write(submission_src_path, &decoded_code).internal(&ctx)?;

    // create invocation request
    let invoke_task = invoker_api::DbInvokeTask {
        revision: 0,
        run_id: run.id as u32,
        status_update_callback: get_lsu_webhook_url(&ctx, run.id as u32).await,
    };

    let new_inv = NewInvocation::new(&invoke_task).internal(&ctx)?;

    ctx.db().inv_new(new_inv).await.internal(&&ctx)?;

    describe_run(&ctx, &run).await.map(Json)
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct RunPatch {
    /// New score (useful for custom judging)
    #[serde(default)]
    score: Option<i32>,
    /// Queue for judging again
    #[serde(default)]
    rejudge: bool,
}

impl ApiObject for RunPatch {
    fn name() -> &'static str {
        "RunPatch"
    }
}

#[patch("/runs/<id>", data = "<p>")]
pub(crate) async fn route_patch(ctx: Context, id: RunId, p: Json<RunPatch>) -> ApiResult<()> {
    if !ctx.access().wrap_run(id).can_modify_run().await.internal(&ctx)? {
        return Err(ApiError::access_denied(&ctx));
    }
    if p.score.is_some() {
        return Err(ApiError::not_implemented(&ctx));
    }
    let current_run = ctx.db().run_load(id).await.report(&ctx)?;

    let mut patch = db::schema::RunPatch::default();
    if p.rejudge {
        patch.rejudge_id = Some(current_run.rejudge_id + 1);
        // TODO enqueue
    }
    ctx.db().run_update(id, patch).await.internal(&ctx)?;

    Ok(())
}

#[delete("/runs/<id>")]
pub(crate) async fn route_delete(ctx: Context, id: RunId) -> ApiResult<rocket::http::Status> {
    if !ctx.access().wrap_run(id).can_modify_run().await.internal(&ctx)? {
        return Err(ApiError::access_denied(&ctx));
    }
    ctx.db().run_delete(id).await.internal(&ctx)?;

    Ok(rocket::http::Status::NoContent)
}

/// Represents Live Status Update
///
/// Some fields can be missing for various reasons, it is normal that particular object will look like {liveScore: null, currentTest: null, finish: false}.
/// Information in all fields except `finish` can be inaccurate, incorrect or outdated.
/// You can rely on following: if `finish` is true, final judging results are available
#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct RunLiveStatusUpdate {
    /// Estimation of score. Usually, final score will be greater than or equal to `live_score`
    live_score: Option<i32>,
    /// Current running test
    current_test: Option<i32>,
    /// Whether final status is available
    finish: bool,
}

impl ApiObject for RunLiveStatusUpdate {
    fn name() -> &'static str {
        "RunLiveStatusUpdate"
    }
}

#[get("/runs/<run_id>/live")]
pub(crate) async fn route_live(ctx: Context, run_id: RunId) -> ApiResult<Json<RunLiveStatusUpdate>> {
    let mut lsu = ctx.global().await;
    let lsu = &mut *lsu;
    let lsu = &mut lsu.live_status_updates;
    let key = crate::global::LsuKey {
        user: ctx.token.user_id(),
        run: run_id as u32,
    };
    let upd = lsu.extract(key);
    debug!("Found update {:?} in cache", &upd);
    if let Some(upd) = upd {
        return Ok(Json(RunLiveStatusUpdate {
            live_score: upd.score,
            current_test: upd.current_test.map(|t| t as i32),
            finish: false,
        }));
    }
    let invocation = ctx.db().inv_last(run_id).await.internal(&ctx)?;
    Ok(Json(RunLiveStatusUpdate {
        live_score: None,
        current_test: None,
        finish: invocation.state().internal(&ctx)?.is_finished(),
    }))
}

#[get("/runs/<id>/source")]
pub(crate) fn route_source(
    ctx: Context,
    id: RunId,
) -> ApiResult<Result<Json<String>, rocket::http::Status>> {
    let source_path = run_data_dir(&ctx, id).join("source");
    let source = std::fs::read(source_path).ok();
    let source = source.as_ref().map(base64::encode);
    Ok(source.ok_or(rocket::http::Status::NoContent).map(Json))
}

#[get("/runs/<id>/binary")]
pub(crate) fn route_binary(
    ctx: Context,
    id: RunId,
) -> ApiResult<Result<Json<String>, rocket::http::Status>> {
    let binary_path = run_data_dir(&ctx, id).join("build");
    let binary = std::fs::read(binary_path).ok();
    let binary = binary.as_ref().map(base64::encode);
    Ok(binary.ok_or(rocket::http::Status::NoContent).map(Json))
}

#[derive(Copy, Clone, rocket::request::FromForm)]
pub(crate) struct RunProtocolFilterParams {
    compile_log: bool,
    test_data: bool,
    output: bool,
    answer: bool,
}

#[get("/runs/<id>/protocol?<filter..>")]
pub(crate)async fn route_protocol(
    ctx: Context,
    id: RunId,
    filter: rocket::request::Form<RunProtocolFilterParams>,
) -> ApiResult<Option<String>> {
    let run_data = run_lookup(&ctx, id).await.map_err(|_| ApiError::not_found(&ctx))?;
    let access_ck = ctx.access().wrap_contest(run_data.contest_id.clone());
    let kind = access_ck.select_judge_log_kind().internal(&ctx)?;
    let path = run_data_dir(&ctx, id)
        .join(format!("inv.{}", run_data.rejudge_id))
        .join(format!("protocol-{}.json", kind.as_str()));
    if !path.exists() {
        return Ok(None);
    }
    debug!("Looking up invocation protocol at {}", path.display());
    let protocol = std::fs::read(path).internal(&ctx)?;

    let protocol: invoker_api::judge_log::JudgeLog =
        serde_json::from_slice(&protocol).internal(&ctx)?;
    let mut protocol = serde_json::to_value(&protocol).internal(&ctx)?;
    filter_protocol(&mut protocol, *filter);
    let protocol = serde_json::to_string(&protocol).internal(&ctx)?;
    Ok(Some(protocol))
}
