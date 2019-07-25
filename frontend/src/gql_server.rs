mod schema;
mod submissions;
mod context;

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

pub(crate) use context::Context;

pub(crate) struct Query;


#[juniper::object(Context = Context)]
impl Query {
    /// Get current API version
    ///
    /// Version returned in format "MAJOR.MINOR"
    /// MAJOR component is incremented, when backwards-incompatible changes were made
    /// MINOR component is incremented, when backwards-compatible changes were made
    ///
    /// It means, that if you developed application with apiVersion X.Y, your application
    /// should assert that MAJOR = X and MINOR >= Y
    fn api_version() -> &str {
        "0.0"
    }

    /// List submissions
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
    /// Submit run
    ///
    /// toolchain: toolchain ID
    /// run_code: run code, base64-encoded
    /// problem: problem ID
    /// contest: contest ID (currently only contest="TODO" is supported)
    fn submit_simple(ctx: &Context, toolchain: schema::ToolchainId, run_code: String, problem: schema::ProblemId, contest: schema::ContestId) -> FieldResult<schema::Run> {
        submissions::submit_simple(ctx, toolchain, run_code, problem, contest)
    }
}

pub(crate) type Schema = juniper::RootNode<'static, Query, Mutation>;
