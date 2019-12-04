use super::{auth, misc, monitor, prelude::*, runs, schema, users, Context, Mutation, Query};

#[juniper::object(Context = Context)]
impl Query {
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

    fn simple_visible_error(ctx: &Context) -> ApiResult<String> {
        let mut ext = ErrorExtension::new();
        ext.set_error_code("SomeError");
        Err(ApiError {
            visible: true,
            extension: ext,
            source: None,
            ctx: ctx.clone(),
        })
    }

    /// List runs
    fn runs(
        ctx: &Context,
        id: Option<schema::RunId>,
        limit: Option<i32>,
    ) -> ApiResult<Vec<runs::Run>> {
        runs::list(ctx, id, limit)
    }

    /// Loads run by id
    fn find_run(ctx: &Context, id: schema::RunId) -> ApiResult<Option<runs::Run>> {
        runs::load(ctx, id)
    }

    /// List toolchains
    fn toolchains(ctx: &Context) -> ApiResult<Vec<schema::Toolchain>> {
        misc::toolchains_list(ctx)
    }

    /// List contests
    fn contests(ctx: &Context) -> ApiResult<Vec<schema::Contest>> {
        misc::get_contests(ctx)
    }

    /// Get standings as JSON-encoded string
    fn standings_simple(ctx: &Context) -> ApiResult<String> {
        monitor::get_standings(ctx)
    }
}

#[juniper::object(Context = Context)]
impl Mutation {
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
    ) -> ApiResult<runs::Run> {
        runs::submit_simple(ctx, toolchain, run_code, problem, contest)
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
    ) -> ApiResult<schema::User> {
        users::create(ctx, login, password, groups)
    }

    /// Modify run
    ///
    /// Depending on arguments provided, two behaviors are possible
    ///
    /// 1) `delete` is set to true.
    /// All other arguments must be unset.
    /// Run will be deleted.
    ///
    /// 2) Update run according to given arguments
    ///
    /// On success, 0 is returned.
    #[graphql(arguments(
        id(description = "Id of run to operate on"),
        status(description = "New status (useful for custom invocation)"),
        rejudge(description = "Queue for invocation again"),
        delete(description = "Delete")
    ))]
    fn modify_run(
        ctx: &Context,
        id: schema::RunId,
        status: Option<runs::InvokeStatusIn>,
        rejudge: Option<bool>,
        delete: Option<bool>,
    ) -> ApiResult<i32> {
        // TODO this return value (i32) is workaround for strange
        runs::modify(ctx, id, status, rejudge, delete).map(|_| 0)
    }

    /// Login using login and password
    ///
    /// See `SessionToken` documentation for more details.
    fn auth_simple(
        ctx: &Context,
        login: String,
        password: String,
    ) -> ApiResult<schema::SessionToken> {
        auth::simple(ctx, login, password)
    }
}
