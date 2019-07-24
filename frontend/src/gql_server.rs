mod schema;

use juniper::FieldResult;

pub(crate) struct Context {
    pub(crate) pool: r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>
}

impl juniper::Context for Context {}

pub(crate) struct Query;

fn describe_submission(submission: &db::schema::Submission) -> schema::Submission {
    use schema::{Submission, SubmissionState};
    Submission {
        id: submission.id(),
        toolchain_name: submission.toolchain.clone(),
        status: frontend_api::JudgeStatus {
            kind: submission.status_kind.clone(),
            code: submission.status.clone(),
        },
        state: match submission.state {
            db::schema::SubmissionState::Done => SubmissionState::Finish,
            db::schema::SubmissionState::Error => SubmissionState::Error,
            db::schema::SubmissionState::Invoke => SubmissionState::Judge,
            db::schema::SubmissionState::WaitInvoke => SubmissionState::Queue,
        },
        score: Some(submission.score),
        problem: submission.problem_name.clone(),
    }
}

#[juniper::object(Context = Context)]
impl Query {
    fn api_version() -> &str {
        "0.0"
    }

    fn submissions(ctx: &Context, id: Option<u32>, limit: Option<u32>) -> FieldResult<schema::Submission> {
        let conn = ctx.pool.get()?;
        let user_submissions = submissions
            .limit(i64::from(params.limit))
            .load::<db::schema::Submission>(&conn)?;
        let user_submissions = user_submissions
            .iter()
            .map(|s| describe_submission(s))
            .collect();
        let res = Ok(user_submissions);
        Ok(res)
    }
}