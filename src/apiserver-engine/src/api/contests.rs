use super::prelude::*;

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct Problem {
    /// Problem title as contestants see, e.g. "Find max flow".
    pub title: String,
    /// Problem name
    pub name: schema::ProblemId,
    /// Problem relative name (aka problem code) as contestants see. This is usually one letter or
    /// something similar, e.g. 'A' or '3F'.
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
    /// Configured by human, something readable like 'olymp-2019', or 'test-contest'
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

#[get("/contests")]
pub(crate) fn route_list(ctx: Context) -> ApiResult<Json<Vec<Contest>>> {
    let res = ctx
        .cfg
        .list::<entity::Contest>()
        .map(describe_contest)
        .collect();
    Ok(Json(res))
}

#[get("/contests/<name>")]
pub(crate) fn route_get(ctx: Context, name: String) -> ApiResult<Json<Option<Contest>>> {
    let res = ctx.cfg.find(&name).map(describe_contest);
    Ok(Json(res))
}

#[get("/contests/<name>/problems")]
pub(crate) fn route_list_problems(ctx: Context, name: String) -> ApiResult<Json<Vec<Problem>>> {
    let contest_cfg: &entity::Contest = match ctx.cfg.find(&name) {
        Some(contest) => contest,
        None => return Err(ApiError::not_found(&ctx)),
    };
    let problems = contest_cfg
        .problems
        .iter()
        .map(|p| Problem {
            title: ctx
                .problem_loader
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

#[get("/contests/<name>/participation")]
pub(crate) async fn route_get_participation(
    ctx: Context,
    name: String,
) -> ApiResult<Json<Participation>> {
    let participation = ctx.load_participation(&name).await?;
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

#[patch("/contests/<name>/participation", data = "<p>")]
pub(crate) async fn route_update_participation(
    ctx: Context,
    name: String,
    p: Json<PartitipationUpdateRequest>,
) -> ApiResult<rocket::http::Status> {
    let resp_ok = rocket::http::Status::NoContent;
    let new_phase = match p.phase.as_deref() {
        Some("ACTIVE") => db::schema::ParticipationPhase::Active,
        Some(_) => return Err(ApiError::new(&ctx, "UnknownParticipationStatus")),
        None => return Ok(resp_ok),
    };
    let contest = match ctx.cfg.find::<entity::Contest>(&name) {
        Some(c) => c,
        None => return Err(ApiError::not_found(&ctx)),
    };
    let access_ck = ctx.access_contest(&name).await?.unwrap();
    if !access_ck.can_participate() {
        return Err(ApiError::access_denied(&ctx));
    }
    let current_participation = ctx.load_participation(&contest.id).await?;

    if current_participation.is_some() {
        return Err(ApiError::new(&ctx, "AlreadyParticipating"));
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
            return Err(ApiError::new(&ctx, "VirtualContestCanNotBeStartedNow"));
        }
        let mut p = db::schema::NewParticipation::default();
        p.set_virtual_contest_start_time(Some(chrono::Utc::now()));
        p
    };
    new_participation.contest_id = contest.id.clone();
    new_participation.user_id = ctx.token.user_id();
    new_participation.set_phase(new_phase);

    ctx.db().part_new(new_participation).await.internal(&ctx)?;
    Ok(resp_ok)
}
