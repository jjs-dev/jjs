mod context;
mod schema;
mod submissions;
mod users;

use juniper::FieldResult;
use std::marker::PhantomData;

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

pub(crate) use context::{Context, ContextFactory};

pub(crate) struct Query<'a>(pub PhantomData<&'a ()>);

#[juniper::object(Context = Context)]
impl<'a> Query<'a> {
    /// Get current API version
    ///
    /// Version returned in format "MAJOR.MINOR".
    /// MAJOR component is incremented, when backwards-incompatible changes were made.
    /// MINOR component is incremented, when backwards-compatible changes were made.
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

pub(crate) struct Mutation<'a>(pub PhantomData<&'a ()>);

#[juniper::object(Context = Context)]
impl<'a> Mutation<'a> {
    /// Submit run
    #[graphql(arguments(
        toolchain(description = "toolchain ID"),
        run_code(description = "run code, base64-encoded"),
        problem(description = "problem ID"),
        contest(description = "contest ID (currently only contest=\"TODO\" is supported)")
    ))]
    fn submit_simple(
        ctx: &Context,
        toolchain: schema::ToolchainId,
        run_code: String,
        problem: schema::ProblemId,
        contest: schema::ContestId,
    ) -> FieldResult<schema::Run> {
        submissions::submit_simple(ctx, toolchain, run_code, problem, contest)
    }

    /// Creates new user
    #[graphql(arguments(
        login(description = "login"),
        password(description = "Password (no strength validation is performed)"),
        groups(description = "List of groups new user should belong to")
    ))]
    fn create_user(
        ctx: &Context,
        login: String,
        password: String,
        groups: Vec<String>,
    ) -> FieldResult<schema::User> {
        users::create(ctx, login, password, groups)
    }
}

pub(crate) type Schema = juniper::RootNode<'static, Query<'static>, Mutation<'static>>;
