use super::{
    prelude::*,
    schema::{Run, RunId},
};

fn describe_submission(submission: &db::schema::Run) -> Run {
    Run {
        id: submission.id,
        toolchain_name: submission.toolchain_id.clone(),
        status: schema::InvokeStatusOut {
            kind: submission.status_kind.clone(),
            code: submission.status_code.clone(),
        },
        score: Some(submission.score),
        problem: submission.problem_id.clone(),
    }
}

pub(super) fn list(ctx: &Context, id: Option<RunId>, limit: Option<i32>) -> ApiResult<Vec<Run>> {
    let user_submissions = ctx.db.run_select(id, limit.map(|x| x as u32))?;
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
) -> ApiResult<Run> {
    use db::schema::NewInvocationRequest;
    let toolchain = ctx.cfg.toolchains.get(toolchain as usize);
    let toolchain = match toolchain {
        Some(tc) => tc.clone(),
        None => return "unknown toolchain".report(),
    };
    if contest != "TODO" {
        return "unknown contest".report();
    }

    let problem = ctx.cfg.contests[0]
        .problems
        .iter()
        .find(|pr| pr.code == problem)
        .cloned();
    let problem = match problem {
        Some(p) => p,
        None => return "unknown problem".report(),
    };
    let prob_name = problem.name.clone();

    let new_run = db::schema::NewRun {
        toolchain_id: toolchain.name,
        status_code: "QUEUE_JUDGE".to_string(),
        status_kind: "QUEUE".to_string(),
        problem_id: prob_name,
        score: 0,
        rejudge_id: 1,
    };

    let run = ctx.db.run_new(new_run)?;

    // Put run in sysroot
    let run_dir = ctx
        .cfg
        .sysroot
        .join("var/submissions")
        .join(&format!("s-{}", run.id));
    std::fs::create_dir(&run_dir)?;
    let submission_src_path = run_dir.join("source");
    let decoded_code = base64::decode(&code).report()?;
    std::fs::write(submission_src_path, &decoded_code)?;

    // create invocation request
    let new_inv_req = NewInvocationRequest {
        invoke_revision: 0,
        run_id: run.id,
    };

    ctx.db.inv_req_new(new_inv_req)?;

    Ok(describe_submission(&run))
}

pub(super) fn modify(
    ctx: &Context,
    id: RunId,
    status: Option<schema::InvokeStatusIn>,
    rejudge: Option<bool>,
    delete: Option<bool>,
) -> ApiResult<()> {
    let should_delete = delete.unwrap_or(false);
    if should_delete {
        if status.is_some() || rejudge.is_some() {
            return "both modification and delete were requested".report();
        }
        ctx.db.run_delete(id)?;
    } else {
        let mut patch = db::schema::RunPatch::default();
        if let Some(new_status) = status {
            patch.status_kind = Some(new_status.kind);
            patch.status_code = Some(new_status.code);
        }
        // TODO: handle rejudge
        ctx.db.run_update(id, patch)?;
    }

    Ok(())
}
