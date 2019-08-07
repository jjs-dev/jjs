#![feature(proc_macro_hygiene, decl_macro, type_alias_enum_variants, param_attrs)]

#[macro_use]
extern crate rocket;

mod config;
mod gql_server;
mod password;
mod root_auth;
mod security;

use cfg::Config;
use diesel::prelude::*;
use rocket::{fairing::AdHoc, http::Status, State};
use rocket_contrib::json::Json;
use security::{AccessCheckService, SecretKey, Token};
use slog::Logger;
use std::{fmt::Debug, marker::PhantomData};

#[derive(Debug)]
enum FrontendError {
    Internal(Option<Box<dyn Debug>>),
    Db(diesel::result::Error),
    DbConn(r2d2::Error),
}

impl<'r> rocket::response::Responder<'r> for FrontendError {
    fn respond_to(self, _request: &rocket::Request) -> rocket::response::Result<'r> {
        eprintln!("FrontendError: {:?}", &self);
        let res = match self {
            FrontendError::Internal(_) | FrontendError::Db(_) | FrontendError::DbConn(_) => {
                Status::InternalServerError
            }
        };
        Err(res)
    }
}

impl From<diesel::result::Error> for FrontendError {
    fn from(e: diesel::result::Error) -> Self {
        FrontendError::Db(e)
    }
}

impl From<r2d2::Error> for FrontendError {
    fn from(e: r2d2::Error) -> Self {
        FrontendError::DbConn(e)
    }
}

type Response<R> = Result<Json<R>, FrontendError>;

type DbPool = r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>;

#[catch(400)]
fn catch_bad_request() -> &'static str {
    r#"
Your request is incorrect.
Possible reasons:
- Query body is missing or is not valid JSON
- X-Jjs-Auth header is missing or is not valid access token
    "#
}

#[post("/auth/anonymous")]
fn route_auth_anonymous(
    secret_key: State<SecretKey>,
) -> Response<Result<frontend_api::AuthToken, frontend_api::CommonError>> {
    let tok = Token::new_guest();

    let buf = tok.serialize(&secret_key.0);
    let res = Ok(frontend_api::AuthToken { buf });

    Ok(Json(res))
}

#[post("/auth/simple", data = "<data>")]
fn route_auth_simple(
    data: Json<frontend_api::AuthSimpleParams>,
    secret_key: State<SecretKey>,
    db_pool: State<DbPool>,
) -> Response<Result<frontend_api::AuthToken, frontend_api::AuthSimpleError>> {
    let conn = db_pool.get()?;
    let succ = {
        use db::schema::users::dsl::*;

        let user = users
            .filter(username.eq(&data.0.login))
            .load::<db::schema::User>(&conn)?;
        if !user.is_empty() {
            let us = &user[0];
            password::check_password_hash(data.0.password.as_str(), us.password_hash.as_str())
        } else {
            false
        }
    };
    let res = if succ {
        let tok = Token::issue_for_user(&data.login, &conn);
        Ok(frontend_api::AuthToken {
            buf: tok.serialize(&secret_key.0),
        })
    } else {
        Err(frontend_api::AuthSimpleError::IncorrectPassword)
    };

    Ok(Json(res))
}

#[post("/submissions/modify", data = "<params>")]
fn route_submissions_set_info(
    params: Json<frontend_api::SubmissionsSetInfoParams>,
    db: State<DbPool>,
    access: AccessCheckService,
) -> Response<Result<(), frontend_api::CommonError>> {
    use db::schema::runs::dsl::*;
    if !access.to_access_checker().can_manage_submissions() {
        let res = Err(frontend_api::CommonError::AccessDenied);
        return Ok(Json(res));
    }
    let conn = db.get()?;
    let should_delete = params.delete;
    if should_delete {
        diesel::delete(runs)
            .filter(id.eq(params.id as i32))
            .execute(&conn)?;
    } else {
        let mut changes: db::schema::RunPatch = Default::default();
        if let Some(new_status) = &params.status {
            changes.status_code = Some(new_status.code.to_string());
            changes.status_kind = Some(new_status.kind.to_string());
        }
        diesel::update(runs)
            .filter(id.eq(params.id as i32))
            .set(changes)
            .execute(&conn)?;
    }
    let res = Ok(());
    Ok(Json(res))
}

fn submissions_blob(
    _params: Json<frontend_api::SubmissionsBlobParams>,
    _db: State<DbPool>,
) -> Response<Result<frontend_api::Blob, frontend_api::CommonError>> {
    unimplemented!()
}

#[post("/toolchains/list")]
fn route_toolchains_list(
    cfg: State<Config>,
) -> Response<Result<Vec<frontend_api::ToolchainInformation>, frontend_api::CommonError>> {
    let res = cfg
        .toolchains
        .iter()
        .enumerate()
        .map(|(i, tc)| frontend_api::ToolchainInformation {
            name: tc.name.clone(),
            id: i as frontend_api::ToolchainId,
        })
        .collect();
    let res = Ok(res);

    Ok(Json(res))
}

fn describe_problem(problem: &cfg::Problem) -> frontend_api::ProblemInformation {
    frontend_api::ProblemInformation {
        code: problem.code.clone(),
        title: "TBD".to_string(),
    }
}

fn describe_contest(contest: &cfg::Contest, long_form: bool) -> frontend_api::ContestInformation {
    frontend_api::ContestInformation {
        title: contest.title.clone(),
        name: "TODO".to_string(),
        problems: if long_form {
            Some(
                contest
                    .problems
                    .iter()
                    .map(|p| describe_problem(p))
                    .collect(),
            )
        } else {
            None
        },
    }
}

// FIXME: check VIEW right
#[post("/contests/list", data = "<_params>")]
fn route_contests_list(
    _params: Json<frontend_api::EmptyParams>,
    cfg: State<Config>,
) -> Response<Result<Vec<frontend_api::ContestInformation>, frontend_api::CommonError>> {
    let data = cfg
        .contests
        .iter()
        .map(|c| frontend_api::ContestInformation {
            title: c.title.clone(),
            name: "TODO".to_string(),
            problems: None, // it is short form
        })
        .collect();
    let res = Ok(data);

    Ok(Json(res))
}

#[post("/contests/describe", data = "<params>")]
fn route_contests_describe(
    access: AccessCheckService,
    params: Json<frontend_api::ContestId>,
    cfg: State<Config>,
) -> Response<Result<frontend_api::ContestInformation, frontend_api::CommonError>> {
    if params.into_inner().as_str() != "TODO" {
        let res = Err(frontend_api::CommonError::NotFound);
        return Ok(Json(res));
    }

    if !access.to_access_checker().can_view_contest() {
        let res = Err(frontend_api::CommonError::NotFound);
        return Ok(Json(res));
    }

    let data = describe_contest(&cfg.contests[0], true);
    let res = Ok(data);
    Ok(Json(res))
}

#[get("/")]
fn route_ping() -> &'static str {
    "JJS frontend: pong"
}

#[get("/graphiql")]
fn route_graphiql() -> rocket::response::content::Html<String> {
    juniper_rocket::graphiql_source("/graphql")
}

#[rocket::get("/graphql?<request>")]
fn route_get_graphql(
    ctx: gql_server::Context,
    request: juniper_rocket::GraphQLRequest,
    schema: State<gql_server::Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &ctx)
}

#[rocket::post("/graphql", data = "<request>")]
fn route_post_graphql(
    ctx: gql_server::Context,
    request: juniper_rocket::GraphQLRequest,
    schema: State<gql_server::Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &ctx)
}

#[derive(Clone)]
struct GqlApiSchema(String);

#[rocket::get("/graphql/schema")]
fn route_get_graphql_schema(schema: State<GqlApiSchema>) -> String {
    schema.clone().0
}

fn launch_api(frcfg: &config::FrontendConfig, logger: &Logger, config: &cfg::Config) {
    let postgress_url =
        std::env::var("DATABASE_URL").expect("'DATABASE_URL' environment variable is not set");
    let pg_conn_manager =
        diesel::r2d2::ConnectionManager::<diesel::pg::PgConnection>::new(postgress_url);
    let pool = r2d2::Pool::new(pg_conn_manager).expect("coudln't initialize DB connection pool");

    let cfg1 = frcfg.clone();
    let cfg2 = frcfg.clone();

    let security_data = security::init(&config);

    if std::env::var("JJS_FRONTEND_DBG_DUMP_ACL").is_ok() {
        println!("security configs: {:?}", &security_data);
    }

    let rocket_cfg_env = match frcfg.env {
        config::Env::Prod => rocket::config::Environment::Production,
        config::Env::Dev => rocket::config::Environment::Development,
    };
    let mut rocket_config = rocket::Config::new(rocket_cfg_env);

    rocket_config.set_address(frcfg.host.clone()).unwrap();
    rocket_config.set_port(frcfg.port);
    rocket_config.set_log_level(match frcfg.env {
        config::Env::Dev => rocket::config::LoggingLevel::Normal,
        config::Env::Prod => rocket::config::LoggingLevel::Critical,
    });
    rocket_config
        .set_secret_key(base64::encode(&frcfg.secret))
        .unwrap();

    let graphql_context_factory = gql_server::ContextFactory {
        pool: pool.clone(),
        cfg: std::sync::Arc::new(config.clone()),
    };

    let graphql_schema = gql_server::Schema::new(
        gql_server::Query(PhantomData),
        gql_server::Mutation(PhantomData),
    );

    let (intro_data, intro_errs) = juniper::introspect(
        &graphql_schema,
        &graphql_context_factory.create_context_unrestricted(),
        juniper::IntrospectionFormat::default(),
    )
    .unwrap();
    assert!(intro_errs.is_empty());

    let introspection_json = serde_json::to_string(&intro_data).unwrap();

    rocket::custom(rocket_config)
        .manage(pool)
        .manage(graphql_context_factory)
        .manage(graphql_schema)
        .manage(config.clone())
        .manage(logger.clone())
        .manage(security_data)
        .manage(GqlApiSchema(introspection_json))
        .attach(AdHoc::on_attach("ProvideSecretKey", move |rocket| {
            Ok(rocket.manage(SecretKey(cfg1.secret.clone())))
        }))
        .attach(AdHoc::on_attach("RegisterEnvironmentKind", move |rocket| {
            Ok(rocket.manage(cfg2.env))
        }))
        .mount(
            "/",
            routes![
                route_get_graphql_schema,
                route_graphiql,
                route_get_graphql,
                route_post_graphql,
                route_ping,
            ],
        )
        .register(catchers![catch_bad_request])
        .launch();
}

fn launch_root_login_server(logger: &slog::Logger, cfg: &config::FrontendConfig) {
    use std::sync::Arc;
    let key = cfg.secret.clone();
    let cfg = root_auth::Config {
        socket_path: String::from("/tmp/jjs-auth-sock"), // FIXME dehardcode
        token_provider: Arc::new(move || security::Token::new_root().serialize(&key)),
    };
    let sublogger = logger.new(slog::o!("app" => "jjs:frontend:localauth"));
    root_auth::start(sublogger, cfg.clone());
}

fn main() {
    use slog::Drain;
    dotenv::dotenv().ok();
    let frontend_cfg = config::FrontendConfig::obtain();
    let cfg = cfg::get_config();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, slog::o!("app"=>"jjs:frontend"));
    slog::info!(logger, "starting");

    launch_root_login_server(&logger, &frontend_cfg);
    launch_api(&frontend_cfg, &logger, &cfg);
}
