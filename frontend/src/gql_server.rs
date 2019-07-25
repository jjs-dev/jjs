mod schema;
mod submissions;

use juniper::FieldResult;

struct InternalError(Box<dyn std::error::Error>);

impl<E: std::error::Error + 'static> From<E> for InternalError {
    fn from(e: E) -> InternalError {
        InternalError(Box::new(e))
    }
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub(crate) struct Context {
    pub(crate) pool: r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>,
    pub(crate) cfg: cfg::Config,
}

impl juniper::Context for Context {}

pub(crate) struct Query;


#[juniper::object(Context = Context)]
impl Query {
    fn api_version() -> &str {
        "0.0"
    }

    fn submissions(
        ctx: &Context,
        id: Option<i32>,
        limit: Option<i32>,
    ) -> FieldResult<Vec<schema::Run>> {
        submissions::list(ctx, id, limit)
    }
}

pub(crate) struct Mutation;

#[juniper::object(Context = Context)]
impl Mutation {
    fn submit_simple(ctx: &Context, toolchain: schema::ToolchainId, run_code: String, problem: schema::ProblemId, contest: schema::ContestId) -> FieldResult<schema::Run> {
        submissions::submit_simple(ctx, toolchain, run_code, problem, contest)
    }
}

pub(crate) type Schema = juniper::RootNode<'static, Query, Mutation>;
