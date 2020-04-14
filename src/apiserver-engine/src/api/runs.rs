use super::{
    prelude::*,
    schema::{ContestId, RunId},
};
use futures::stream::{StreamExt, TryStreamExt};
use log::debug;
use std::path::PathBuf;

pub(crate) fn register_routes(c: &mut web::ServiceConfig) {
    c.route("/runs", web::get().to(route_list))
        .route("/runs/{id}", web::get().to(route_get))
        .route("/runs", web::post().to(route_submit_simple))
        .route("/runs/{id}", web::patch().to(route_patch))
        .route("/runs/{id}", web::delete().to(route_delete))
        .route("/runs/{id}/live", web::get().to(route_live))
        .route("/runs/{id}/source", web::get().to(route_source))
        .route("/runs/{id}/binary", web::get().to(route_binary))
        .route("/runs/{id}/protocol", web::get().to(route_protocol));
}

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

fn run_data_dir(ccx: &ConfigContext, id: RunId) -> PathBuf {
    ccx.data_dir().join("var/runs").join(format!("run.{}", id))
}

async fn run_lookup(cx: &DbContext, id: RunId) -> ApiResult<db::schema::Run> {
    cx.db().run_load(id).await.internal()
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
                    if !filter.resource_usage {
                        test.remove("time_usage");
                        test.remove("memory_usage");
                    }
                }
            }
        }
    }
}

async fn select_judge_log_kind(
    contest: &entity::Contest,
    cx: &SecurityContext,
) -> ApiResult<Option<invoker_api::judge_log::JudgeLogKind>> {
    const ORDER: &[invoker_api::judge_log::JudgeLogKind] = &[
        invoker_api::judge_log::JudgeLogKind::Full,
        invoker_api::judge_log::JudgeLogKind::Contestant,
    ];
    for &kind in ORDER {
        let outcome = cx
            .access()
            .with_action(Action::Get)
            .with_resource_kind(ResourceKind::RUN_PROTOCOL)
            .with_conditions(make_conditions![contest.id.clone()])
            .try_authorize()
            .await?;
        if outcome.is_allow() {
            return Ok(Some(kind));
        }
    }
    Ok(None)
}
async fn describe_run(
    db_cx: &DbContext,
    scx: &SecurityContext,
    ecx: &EntityContext,
    run: &db::schema::Run,
) -> ApiResult<Run> {
    let last_inv = db_cx.db().inv_last(run.id).await.internal()?;
    let contest = match ecx.entities().find(&run.contest_id) {
        Some(c) => c,
        None => return Err(ApiError::not_found()),
    };
    //scx.access().authorize().await?;
    let kind = select_judge_log_kind(contest, scx).await?;
    let kind = match kind {
        Some(k) => k,
        None => return Err(ApiError::access_denied()),
    };
    let inv_out_header = last_inv
        .invoke_outcome_headers()
        .internal()?
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
#[derive(Deserialize)]
struct RouteListQueryParams {
    limit: Option<i32>,
    user: Option<String>,
}

async fn route_list(
    db_cx: DbContext,
    sec_cx: SecurityContext,
    ent_cx: EntityContext,
    query_params: web::Query<RouteListQueryParams>,
) -> ApiResult<Json<Vec<Run>>> {
    let RouteListQueryParams { limit, user } = query_params.into_inner();

    let user_id = match user.as_deref() {
        Some(s) => match db_cx.db().user_try_load_by_login(s).await.internal()? {
            Some(user) => Some(user.id),
            None => return Err(ApiError::new("UserNotFound")),
        },
        None => None,
    };
    {
        let builder = sec_cx.access().with_action(Action::List);

        match user_id {
            Some(user_id) => {
                builder
                    .with_resource_kind(ResourceKind::USER_RUNS_LIST)
                    .with_conditions(make_conditions![resource_ident::UserId::new(user_id)])
                    .authorize()
                    .await?;
            }
            None => {
                builder
                    .with_resource_kind(ResourceKind::RUNS_LIST)
                    .with_conditions(make_conditions![])
                    .authorize()
                    .await?;
            }
        }
    }
    let runs = db_cx
        .db()
        .run_select(user_id, limit.map(|x| x as u32))
        .await
        .internal()?;
    Ok(Json(
        futures::stream::iter(runs.iter())
            .then(|s| describe_run(&db_cx, &sec_cx, &ent_cx, s))
            .try_collect::<Vec<_>>()
            .await?,
    ))
}
async fn check_run_exists(db_cx: &DbContext, run_id: RunId) -> ApiResult<()> {
    let run = db_cx.db().run_try_load(run_id).await.internal()?;
    match run {
        Some(_) => Ok(()),
        None => Err(ApiError::not_found()),
    }
}

pub(crate) async fn route_get(
    db_cx: DbContext,
    sec_cx: SecurityContext,
    ent_cx: EntityContext,
    path_params: web::Path<i32>,
) -> ApiResult<Json<Run>> {
    let id = path_params.into_inner();
    check_run_exists(&db_cx, id).await?;
    let db_run = db_cx.db().run_load(id).await.internal()?;
    sec_cx
        .access()
        .with_conditions(make_conditions![resource_ident::ContestId::new(
            db_run.contest_id.clone()
        )])
        .with_action(Action::Get)
        .with_resource_kind(ResourceKind::RUN)
        .authorize()
        .await?;
    Ok(Json(describe_run(&db_cx, &sec_cx, &ent_cx, &db_run).await?))
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

pub(crate) async fn route_submit_simple(
    ecx: EntityContext,
    scx: SecurityContext,
    ccx: CredentialsContext,
    dcx: DbContext,
    cfg_cx: ConfigContext,

    p: Json<RunSimpleSubmitParams>,
) -> ApiResult<Json<Run>> {
    let toolchain = ecx.entities().find::<entity::Toolchain>(&p.toolchain);
    let toolchain = match toolchain {
        Some(tc) => tc.clone(),
        None => return Err(ApiError::new("ToolchainUnknown")),
    };
    let contest: &entity::Contest = match ecx.entities().find(&p.contest) {
        Some(ent) => ent,
        None => return Err(ApiError::new("ContestUnknown")),
    };
    scx.access()
        .with_conditions(make_conditions![resource_ident::ContestId::new(
            contest.id.clone()
        )])
        .with_resource_kind(ResourceKind::RUN)
        .with_action(Action::Create)
        .authorize()
        .await?;
    let problem = contest
        .problems
        .iter()
        .find(|pr| pr.code == p.problem)
        .cloned();
    let problem = match problem {
        Some(p) => p,
        None => return Err(ApiError::new("ProblemUnknown")),
    };
    let prob_name = problem.name;

    let new_run = db::schema::NewRun {
        toolchain_id: toolchain.name,
        problem_id: prob_name,
        rejudge_id: 0,
        user_id: ccx.token().user_info.id,
        contest_id: contest.id.to_string(),
    };

    let run = dcx.db().run_new(new_run).await.internal()?;

    // Put run in sysroot
    let run_dir = cfg_cx
        .data_dir()
        .join("var/runs")
        .join(&format!("run.{}", run.id));
    tokio::fs::create_dir(&run_dir).await.internal()?;
    let submission_src_path = run_dir.join("source");
    let decoded_code = base64::decode(&p.code).report()?;
    tokio::fs::write(submission_src_path, &decoded_code)
        .await
        .internal()?;

    // create invocation request
    let invoke_task = invoker_api::DbInvokeTask {
        revision: 0,
        run_id: run.id as u32,
    };

    let new_inv = db::schema::NewInvocation::new(&invoke_task).internal()?;

    dcx.db().inv_new(new_inv).await.internal()?;

    describe_run(&dcx, &scx, &ecx, &run).await.map(Json)
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

async fn route_patch(
    dcx: DbContext,
    scx: SecurityContext,
    path_params: web::Path<RunId>,
    p: Json<RunPatch>,
) -> ApiResult<EmptyResponse> {
    let id = path_params.into_inner();
    if p.score.is_some() {
        return Err(ApiError::not_implemented());
    }
    let current_run = dcx.db().run_try_load(id).await.internal()?;
    let current_run = match current_run {
        Some(run) => run,
        None => return Err(ApiError::not_found()),
    };
    scx.access()
        .with_conditions(make_conditions![resource_ident::ContestId::new(
            current_run.contest_id.clone()
        )])
        .with_action(Action::Patch)
        .with_resource_kind(ResourceKind::RUN)
        .authorize()
        .await?;

    let mut patch = db::schema::RunPatch::default();
    if p.rejudge {
        patch.rejudge_id = Some(current_run.rejudge_id + 1);
        // TODO enqueue
    }
    dcx.db().run_update(id, patch).await.internal()?;

    Ok(EmptyResponse)
}

async fn route_delete(
    scx: SecurityContext,
    dcx: DbContext,
    path_params: web::Path<RunId>,
) -> ApiResult<EmptyResponse> {
    let id = path_params.into_inner();
    let run_data = dcx.db().run_try_load(id).await.internal()?;
    let run_data = match run_data {
        Some(d) => d,
        None => return Err(ApiError::not_found()),
    };
    scx.access()
        .with_conditions(make_conditions![resource_ident::ContestId::new(
            run_data.contest_id.clone()
        )])
        .with_action(Action::Delete)
        .with_resource_kind(ResourceKind::RUN)
        .authorize()
        .await?;
    dcx.db().run_delete(id).await.internal()?;

    Ok(EmptyResponse)
}

/// Represents Live Status Update
///
/// Some fields can be missing for various reasons, it is normal that particular
/// object will look like {liveScore: null, currentTest: null, finish: false}.
/// Information in all fields except `finish` can be inaccurate, incorrect or
/// outdated. You can rely on following: if `finish` is true, final judging
/// results are available
#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct RunLiveStatusUpdate {
    /// Estimation of score. Usually, final score will be greater than or equal
    /// to `live_score`
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

pub(crate) async fn route_live(
    dcx: DbContext,
    path_params: web::Path<RunId>,
) -> ApiResult<Json<RunLiveStatusUpdate>> {
    let run_id = path_params.into_inner();
    let lsu_key = format!("lsu-{}", run_id);
    let lsu: Option<invoker_api::LiveStatusUpdate> = dcx.db().kv_get(&lsu_key).await.internal()?;

    debug!(
        "Found update {:?} in KV storage, with key {}",
        &lsu, lsu_key
    );
    if let Some(upd) = lsu {
        dcx.db().kv_del(&lsu_key).await.internal()?;
        return Ok(Json(RunLiveStatusUpdate {
            live_score: upd.score,
            current_test: upd.current_test.map(|t| t as i32),
            finish: false,
        }));
    }
    let invocation = dcx.db().inv_last(run_id).await.internal()?;
    Ok(Json(RunLiveStatusUpdate {
        live_score: None,
        current_test: None,
        finish: invocation.state().internal()?.is_finished(),
    }))
}

async fn route_source(
    scx: SecurityContext,
    ccx: ConfigContext,
    dcx: DbContext,
    path_params: web::Path<RunId>,
) -> ApiResult<actix_web::Either<Json<String>, EmptyResponse>> {
    let id = path_params.into_inner();
    let run_data = dcx.db().run_try_load(id).await.internal()?;
    let run_data = match run_data {
        Some(d) => d,
        None => return Err(ApiError::not_found()),
    };
    scx.access()
        .with_conditions(make_conditions![resource_ident::ContestId::new(
            run_data.contest_id.clone()
        )])
        .with_action(Action::Get)
        .with_resource_kind(ResourceKind::RUN)
        .authorize()
        .await?;
    let source_path = run_data_dir(&ccx, id).join("source");
    let source = tokio::fs::read(source_path).await.ok();
    let source = source.as_ref().map(base64::encode);
    let source = match source {
        Some(s) => actix_web::Either::A(Json(s)),
        None => actix_web::Either::B(EmptyResponse),
    };
    Ok(source)
}

async fn route_binary(
    scx: SecurityContext,
    ccx: ConfigContext,
    dcx: DbContext,
    path_params: web::Path<RunId>,
) -> ApiResult<actix_web::Either<Json<String>, EmptyResponse>> {
    let id = path_params.into_inner();
    let run_data = dcx.db().run_try_load(id).await.internal()?;
    let run_data = match run_data {
        Some(d) => d,
        None => return Err(ApiError::not_found()),
    };
    scx.access()
        .with_conditions(make_conditions![resource_ident::ContestId::new(
            run_data.contest_id.clone()
        )])
        .with_action(Action::Get)
        .with_resource_kind(ResourceKind::RUN)
        .authorize()
        .await?;

    let binary_path = run_data_dir(&ccx, id).join("build");
    let binary = tokio::fs::read(binary_path).await.ok();
    let binary = binary.as_ref().map(base64::encode);
    let binary = match binary {
        Some(b) => actix_web::Either::A(Json(b)),
        None => actix_web::Either::B(EmptyResponse),
    };
    Ok(binary)
}

#[derive(Copy, Clone, Deserialize)]
pub(crate) struct RunProtocolFilterParams {
    compile_log: bool,
    test_data: bool,
    output: bool,
    answer: bool,
    resource_usage: bool,
}

pub(crate) async fn route_protocol(
    scx: SecurityContext,
    ccx: ConfigContext,
    ecx: EntityContext,
    dcx: DbContext,
    path_params: web::Path<RunId>,
    filter: web::Query<RunProtocolFilterParams>,
) -> ApiResult<Option<String>> {
    let id = path_params.into_inner();
    let run_data = run_lookup(&dcx, id).await?;
    scx.access()
        .with_conditions(make_conditions![resource_ident::ContestId::new(
            run_data.contest_id.clone()
        )])
        .with_action(Action::Get)
        .with_resource_kind(ResourceKind::RUN_PROTOCOL)
        .authorize()
        .await?;
    let contest = ecx
        .entities()
        .find(&run_data.contest_id)
        .ok_or_else(|| ApiError::new("ContestGone"))?;
    let kind = select_judge_log_kind(contest, &scx)
        .await?
        .ok_or_else(ApiError::access_denied)?;
    let path = run_data_dir(&ccx, id)
        .join(format!("inv.{}", run_data.rejudge_id))
        .join(format!("protocol-{}.json", kind.as_str()));
    if !path.exists() {
        return Ok(None);
    }
    debug!("Looking up invocation protocol at {}", path.display());
    let protocol = std::fs::read(path).internal()?;

    let protocol: invoker_api::judge_log::JudgeLog =
        serde_json::from_slice(&protocol).internal()?;
    let mut protocol = serde_json::to_value(&protocol).internal()?;
    filter_protocol(&mut protocol, *filter);
    let protocol = serde_json::to_string(&protocol).internal()?;
    Ok(Some(protocol))
}
