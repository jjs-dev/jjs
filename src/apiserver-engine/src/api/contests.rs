pub(crate) fn register_routes(c: &mut web::ServiceConfig) {
    c.route("/contests", web::get().to(route_list))
        .route("/contests/{name}", web::get().to(route_get))
        .route(
            "/contests/{name}/problems",
            web::get().to(route_list_problems),
        )
        .route(
            "/contests/{name}/participation",
            web::get().to(route_get_participation),
        )
        .route(
            "/contests/{name}/participation",
            web::patch().to(route_update_participation),
        );
}

use super::prelude::*;

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct Problem {
    /// Problem title as contestants see, e.g. "Find max flow".
    pub title: String,
    /// Problem name
    pub name: schema::ProblemId,
    /// Problem relative name (aka problem code) as contestants see. This is
    /// usually one letter or something similar, e.g. 'A' or '3F'.
    pub rel_name: schema::ProblemId,
}

impl ApiObject for Problem {
    fn name() -> &'static str {
        "Problem"
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct Contest {
    /// E.g. "Berlandian Olympiad in Informatics. Finals. Day 3."
    pub title: String,
    /// Configured by human, something readable like 'olymp-2019', or
    /// 'test-contest'
    pub id: schema::ContestId,
}

impl ApiObject for Contest {
    fn name() -> &'static str {
        "Contest"
    }
}

fn describe_contest(c: &entity::Contest) -> Contest {
    Contest {
        title: c.title.clone(),
        id: c.id.clone(),
    }
}

pub(crate) async fn route_list(cx: EntityContext) -> ApiResult<Json<Vec<Contest>>> {
    let res = cx
        .entities()
        .list::<entity::Contest>()
        .map(describe_contest)
        .collect();
    Ok(Json(res))
}

pub(crate) async fn route_get(cx: EntityContext, name: String) -> ApiResult<Json<Option<Contest>>> {
    let res = cx.entities().find(&name).map(describe_contest);
    Ok(Json(res))
}

pub(crate) async fn route_list_problems(
    ctx: EntityContext,
    name: String,
) -> ApiResult<Json<Vec<Problem>>> {
    let contest_cfg: &entity::Contest = match ctx.entities().find(&name) {
        Some(contest) => contest,
        None => return Err(ApiError::not_found()),
    };
    let problems = contest_cfg
        .problems
        .iter()
        .map(|p| Problem {
            title: ctx
                .problems()
                .find(&p.name)
                .expect("problem not found")
                .0
                .title
                .clone(),
            rel_name: p.code.clone(),
            name: p.name.clone(),
        })
        .collect();
    Ok(Json(problems))
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct Participation {
    pub phase: String,
}

impl ApiObject for Participation {
    fn name() -> &'static str {
        "Participation"
    }
}

fn stringify_participation_phase(phase: db::schema::ParticipationPhase) -> &'static str {
    match phase {
        db::schema::ParticipationPhase::Active => "ACTIVE",
        db::schema::ParticipationPhase::__Last => unreachable!(),
    }
}

pub(crate) async fn route_get_participation(
    dbcx: DbContext,
    ccx: CredentialsContext,
    name: String,
) -> ApiResult<Json<Participation>> {
    let participation = load_participation(&dbcx, &ccx, &name).await?;
    let desc = match participation {
        Some(p) => stringify_participation_phase(p.phase()),
        None => "MISSING",
    };
    Ok(Json(Participation {
        phase: desc.to_string(),
    }))
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct PartitipationUpdateRequest {
    pub phase: Option<String>,
}

impl ApiObject for PartitipationUpdateRequest {
    fn name() -> &'static str {
        "ParticipationUpdateRequest"
    }
}

async fn route_update_participation(
    path_params: web::Path<String>,
    p: Json<PartitipationUpdateRequest>,
    db_cx: DbContext,
    cred_cx: CredentialsContext,
    ecx: EntityContext,
    scx: SecurityContext,
) -> ApiResult<EmptyResponse> {
    let name = path_params.into_inner();
    let new_phase = match p.phase.as_deref() {
        Some("ACTIVE") => db::schema::ParticipationPhase::Active,
        Some(_) => return Err(ApiError::new("UnknownParticipationStatus")),
        None => return Ok(EmptyResponse),
    };
    let contest = match ecx.entities().find::<entity::Contest>(&name) {
        Some(c) => c,
        None => return Err(ApiError::not_found()),
    };
    scx.access()
        .with_conditions(make_conditions![resource_ident::ContestId::new(name)])
        .with_action(Action::Patch)
        .with_resource_kind(ResourceKind::CONTEST)
        .authorize()
        .await?;
    /*let access_ck = scx.access_contest(&name).await?.unwrap();
    if !access_ck.can_participate() {
        return Err(ApiError::access_denied());
    }*/
    let current_participation = load_participation(&db_cx, &cred_cx, &contest.id).await?;

    if current_participation.is_some() {
        return Err(ApiError::new("AlreadyParticipating"));
    }
    let mut new_participation = if !contest.is_virtual {
        // for non-virtual contest, we can easily issue Participation, even if
        // contest is not runnig now: access checker will handle it correcctly
        db::schema::NewParticipation::default()
    } else {
        // for virtual contest, we must check if Participation can be
        // created right now
        let mut registration_open = true;
        let now = chrono::Utc::now();
        if let Some(start) = contest.start_time {
            registration_open = registration_open && (now >= start);
        }
        if let Some(end) = contest.end_time {
            registration_open = registration_open && (now <= end);
        }
        if !registration_open {
            return Err(ApiError::new("VirtualContestCanNotBeStartedNow"));
        }
        let mut p = db::schema::NewParticipation::default();
        p.set_virtual_contest_start_time(Some(chrono::Utc::now()));
        p
    };
    new_participation.contest_id = contest.id.clone();
    new_participation.user_id = cred_cx.token().user_info.id;
    new_participation.set_phase(new_phase);

    db_cx.db().part_new(new_participation).await.internal()?;
    Ok(EmptyResponse)
}

pub(crate) async fn load_participation(
    dcx: &DbContext,
    ccx: &CredentialsContext,
    contest_id: &str,
) -> ApiResult<Option<db::schema::Participation>> {
    dcx.db()
        .part_lookup(ccx.token().user_info.id, &contest_id)
        .await
        .internal()
}
