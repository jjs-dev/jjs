use super::{schema, Context, InternalError};
use diesel::prelude::*;
use juniper::FieldResult;

fn describe_submission(submission: &db::schema::Run) -> schema::Run {
    use schema::Run;
    Run {
        id: submission.id,
        toolchain_name: submission.toolchain_id.clone(),
        status: schema::InvokeStatus {
            kind: submission.status_kind.clone(),
            code: submission.status_code.clone(),
        },
        score: Some(submission.score),
        problem: submission.problem_id.clone(),
    }
}

pub(super) fn list(
    ctx: &Context,
    id: Option<i32>,
    limit: Option<i32>,
) -> FieldResult<Vec<schema::Run>> {
    use db::schema::runs::{dsl, table};
    let conn = ctx.pool.get().map_err(InternalError::from)?;
    let mut query = table.into_boxed();

    if let Some(id) = id {
        query = query.filter(dsl::id.eq(id))
    }

    let user_submissions = query
        .limit(limit.map(i64::from).unwrap_or(i64::max_value()))
        .load::<db::schema::Run>(&conn)
        .map_err(InternalError::from)?;
    let user_submissions = user_submissions
        .iter()
        .map(|s| describe_submission(s))
        .collect();
    Ok(user_submissions)
}

pub(super) fn submit_simple(
    ctx: &Context,
    toolchain: schema::ToolchainId,
    code: String,
    problem: schema::ProblemId,
    contest: schema::ContestId,
) -> FieldResult<schema::Run> {
    use db::schema::{invocation_requests::dsl::*, runs::dsl::*, NewInvocationRequest};
    let toolchain = ctx.cfg.toolchains.get(toolchain as usize);
    let toolchain = match toolchain {
        Some(tc) => tc.clone(),
        None => return Err("unknown toolchain".into()),
    };
    let conn = ctx.pool.get()?;
    if contest != "TODO" {
        return Err("unknown contest".into());
    }

    let problem = ctx.cfg.contests[0]
        .problems
        .iter()
        .find(|pr| pr.code == problem)
        .cloned();
    let problem = match problem {
        Some(p) => p,
        None => return Err("unknown problem".into()),
    };
    let prob_name = problem.name.clone();

    let new_sub = db::schema::NewRun {
        toolchain_id: toolchain.name,
        status_code: "QUEUE_JUDGE".to_string(),
        status_kind: "QUEUE".to_string(),
        problem_id: prob_name,
        score: 0,
        rejudge_id: 1,
    };

    let subm: db::schema::Run = diesel::insert_into(runs)
        .values(&new_sub)
        .get_result(&conn)
        .map_err(InternalError::from)?;

    // Put submission in sysroot
    let submission_dir = ctx
        .cfg
        .sysroot
        .join("var/submissions")
        .join(&format!("s-{}", subm.id));
    std::fs::create_dir(&submission_dir).map_err(InternalError::from)?;
    let submission_src_path = submission_dir.join("source");
    let decoded_code = base64::decode(&code)?;
    std::fs::write(submission_src_path, &decoded_code)?;

    // create invocation request
    let new_inv_req = NewInvocationRequest {
        invoke_revision: 0,
        run_id: subm.id,
    };

    diesel::insert_into(invocation_requests)
        .values(&new_inv_req)
        .execute(&conn)?;

    Ok(describe_submission(&subm))
}
