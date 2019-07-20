#![feature(proc_macro_hygiene, decl_macro, type_alias_enum_variants)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde_derive;

mod config;
mod password;
mod root_auth;
mod security;

use cfg::Config;
use db::schema::{NewSubmission, Submission, SubmissionState};
use diesel::prelude::*;
use rocket::{fairing::AdHoc, http::Status, State};
use rocket_contrib::json::Json;
use security::{AccessCheckService, SecretKey, Token};
use slog::Logger;
use std::fmt::Debug;

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

#[post("/submissions/send", data = "<params>")]
fn route_submissions_send(
    params: Json<frontend_api::SubmissionSendParams>,
    db: State<DbPool>,
    cfg: State<Config>,
) -> Response<Result<frontend_api::SubmissionId, frontend_api::SubmitError>> {
    use db::schema::submissions::dsl::*;
    let toolchain = cfg.toolchains.get(params.toolchain as usize);
    let toolchain = match toolchain {
        Some(tc) => tc.clone(),
        None => {
            let res = Err(frontend_api::SubmitError::UnknownToolchain);
            return Ok(Json(res));
        }
    };
    let conn = db.get().expect("couldn't connect to DB");
    if params.contest != "TODO" {
        let res = Err(frontend_api::SubmitError::UnknownContest);
        return Ok(Json(res));
    }
    let problem = cfg.contests[0]
        .problems
        .iter()
        .find(|pr| pr.code == params.problem)
        .cloned();
    let problem = match problem {
        Some(p) => p,
        None => {
            let res = Err(frontend_api::SubmitError::UnknownProblem);
            return Ok(Json(res));
        }
    };
    let prob_name = problem.name.clone();
    let new_sub = NewSubmission {
        toolchain_id: toolchain.name,
        state: SubmissionState::WaitInvoke,
        status_code: "QUEUE_BUILD".to_string(),
        status_kind: "QUEUE".to_string(),
        problem_name: prob_name,
        score: 0
    };
    let subm: Submission = diesel::insert_into(submissions)
        .values(&new_sub)
        .get_result(&conn)?;
    // Put submission in sysroot
    let submission_dir = cfg
        .sysroot
        .join("var/submissions")
        .join(&format!("s-{}", subm.id()));
    std::fs::create_dir(&submission_dir).expect("Couldn't create submission directory");
    let submission_src_path = submission_dir.join("source");
    let decoded_code =
        match base64::decode(&params.code).map_err(|_e| frontend_api::SubmitError::Base64) {
            Ok(bytes) => bytes,
            Err(e) => return Ok(Json(Err(e))),
        };
    std::fs::write(submission_src_path, &decoded_code)
        .map_err(|e| FrontendError::Internal(Some(Box::new(e))))?;
    let res = Ok(subm.id());
    Ok(Json(res))
}

fn describe_submission(submission: &Submission) -> frontend_api::SubmissionInformation {
    frontend_api::SubmissionInformation {
        id: submission.id(),
        toolchain_name: submission.toolchain.clone(),
        status: frontend_api::JudgeStatus {
            kind: submission.status_kind.clone(),
            code: submission.status.clone(),
        },
        state: match submission.state {
            SubmissionState::Done => frontend_api::SubmissionState::Finish,
            SubmissionState::Error => frontend_api::SubmissionState::Error,
            SubmissionState::Invoke => frontend_api::SubmissionState::Judge,
            SubmissionState::WaitInvoke => frontend_api::SubmissionState::Queue,
        },
        score: Some(submission.score),
        problem: submission.problem_name.clone(),
    }
}

#[post("/submissions/list", data = "<params>")]
fn route_submissions_list(
    params: Json<frontend_api::SubmissionsListParams>,
    db: State<DbPool>,
) -> Response<Result<Vec<frontend_api::SubmissionInformation>, frontend_api::CommonError>> {
    use db::schema::submissions::dsl::*;
    let conn = db.get().expect("Couldn't connect to DB");
    let user_submissions = submissions
        .limit(i64::from(params.limit))
        .load::<Submission>(&conn)?;
    let user_submissions = user_submissions
        .iter()
        .map(|s| describe_submission(s))
        .collect();
    let res = Ok(user_submissions);
    Ok(Json(res))
}

#[post("/submissions/modify", data = "<params>")]
fn route_submissions_set_info(
    params: Json<frontend_api::SubmissionsSetInfoParams>,
    db: State<DbPool>,
    access: AccessCheckService,
) -> Response<Result<(), frontend_api::CommonError>> {
    use db::schema::submissions::dsl::*;
    if !access.to_access_checker().can_manage_submissions() {
        let res = Err(frontend_api::CommonError::AccessDenied);
        return Ok(Json(res));
    }
    let conn = db.get()?;
    let should_delete = params.delete;
    if should_delete {
        diesel::delete(submissions)
            .filter(id.eq(params.id as i32))
            .execute(&conn)?;
    } else {
        let mut changes = db::schema::SubmissionPatch {
            ..Default::default()
        };
        if let Some(new_status) = &params.status {
            changes.status_code = Some(new_status.code.to_string());
            changes.status_kind = Some(new_status.kind.to_string());
        }
        if let Some(new_state) = &params.state {
            changes.state = Some(match new_state {
                frontend_api::SubmissionState::Judge => SubmissionState::Invoke,
                frontend_api::SubmissionState::Queue => SubmissionState::WaitInvoke,
                frontend_api::SubmissionState::Error => SubmissionState::Error,
                frontend_api::SubmissionState::Finish => SubmissionState::Done,
            });
        }
        if params.rejudge {
            changes.state = Some(SubmissionState::WaitInvoke);
        }
        diesel::update(submissions)
            .filter(id.eq(params.id as i32))
            .set(changes)
            .execute(&conn)?;
    }
    let res = Ok(());
    Ok(Json(res))
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

#[post("/users/create", data = "<params>")]
fn route_users_create(
    params: Json<frontend_api::UsersCreateParams>,
    db: State<DbPool>,
    access: AccessCheckService,
) -> Response<Result<(), frontend_api::UsersCreateError>> {
    use db::schema::users::dsl::*;
    if !access.to_access_checker().can_create_users() {
        let res = Err(frontend_api::UsersCreateError::Common(
            frontend_api::CommonError::AccessDenied,
        ));
        return Ok(Json(res));
    }

    let provided_password_hash = password::get_password_hash(params.0.password.as_str());

    let new_user = db::schema::NewUser {
        username: params.login.clone(),
        password_hash: provided_password_hash,
        groups: params.groups.clone(),
    };

    let conn = db.get()?;

    let _user: db::schema::User = diesel::insert_into(users)
        .values(&new_user)
        .get_result(&conn)?;

    let res = Ok(());
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
fn route_api_info() -> String {
    serde_json::to_string(&serde_json::json!({
        "version": "0",
    }))
    .unwrap()
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

    rocket::custom(rocket_config)
        .manage(pool)
        .manage(config.clone())
        .manage(logger.clone())
        .manage(security_data)
        .attach(AdHoc::on_attach("ProvideSecretKey", move |rocket| {
            Ok(rocket.manage(SecretKey(cfg1.secret.clone())))
        }))
        .attach(AdHoc::on_attach("RegisterEnvironmentKind", move |rocket| {
            Ok(rocket.manage(cfg2.env))
        }))
        .mount(
            "/",
            routes![
                route_auth_anonymous,
                route_auth_simple,
                route_submissions_send,
                route_submissions_list,
                route_submissions_set_info,
                route_toolchains_list,
                route_users_create,
                route_contests_describe,
                route_contests_list,
                route_api_info,
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
