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
