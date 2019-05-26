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
use security::{SecretKey, Token};
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
    let succ = {
        use db::schema::users::dsl::*;

        let conn = db_pool.get()?;
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
        let tok = Token::new_for_user(data.login.clone());
        Ok(frontend_api::AuthToken {
            buf: tok.serialize(&secret_key.0),
        })
    } else {
        Err(frontend_api::AuthSimpleError::IncorrectPassword)
    };

    Ok(Json(res))
}

#[post("/submissions/send", data = "<data>")]
fn route_submissions_send(
    data: Json<frontend_api::SubmissionSendParams>,
    db: State<DbPool>,
    cfg: State<Config>,
) -> Response<Result<frontend_api::SubmissionId, frontend_api::SubmitError>> {
    use db::schema::submissions::dsl::*;
    let toolchain = cfg.toolchains.get(data.toolchain as usize);
    let toolchain = match toolchain {
        Some(tc) => tc.clone(),
        None => {
            let res = Err(frontend_api::SubmitError::UnknownToolchain);
            return Ok(Json(res));
        }
    };
    let conn = db.get().expect("couldn't connect to DB");
    let new_sub = NewSubmission {
        toolchain_id: toolchain.name,
        state: SubmissionState::WaitInvoke,
        status: "Queued for compilation".to_string(),
        status_kind: "QUEUE".to_string(),
    };
    let subm: Submission = diesel::insert_into(submissions)
        .values(&new_sub)
        .get_result(&conn)?;
    // Put submission in sysroot
    let submission_dir = format!("{}/var/submissions/s-{}", &cfg.sysroot, subm.id());
    std::fs::create_dir(&submission_dir).expect("Couldn't create submission directory");
    let submission_src_path = format!("{}/source", &submission_dir);
    let decoded_code =
        match base64::decode(&data.code).map_err(|_e| frontend_api::SubmitError::Base64) {
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
        status: submission.status.clone(),
        score: Some(42),
    }
}

#[post("/submissions/list", data = "<params>")]
fn route_submissions_list(
    params: Json<frontend_api::SubmissionsListParams>,
    db: State<DbPool>,
    _token: Token,
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
    token: Token,
    params: Json<frontend_api::UsersCreateParams>,
    db: State<DbPool>,
) -> Response<Result<(), frontend_api::UsersCreateError>> {
    use db::schema::users::dsl::*;
    if !token.is_root() {
        let res = Err(frontend_api::UsersCreateError::Common(
            frontend_api::CommonError::AccessDenied,
        ));
        return Ok(Json(res));
    }

    let provided_password_hash = password::get_password_hash(params.0.password.as_str());

    let new_user = db::schema::NewUser {
        username: params.0.login,
        password_hash: provided_password_hash,
    };

    let conn = db.get()?;

    let _user: db::schema::User = diesel::insert_into(users)
        .values(&new_user)
        .get_result(&conn)?;

    let res = Ok(());
    Ok(Json(res))
}

#[get("/")]
fn route_api_info() -> String {
    serde_json::to_string(&serde_json::json!({
        "version": "0",
    }))
    .unwrap()
}

fn derive_branca_key(secret: &str) -> Vec<u8> {
    use digest::Digest;
    use rand::{Rng, SeedableRng};
    let secret_hash = {
        let mut hasher = sha3::Sha3_512::new();
        hasher.input(secret.as_bytes());
        let result = &hasher.result()[16..48];
        let mut out = [0 as u8; 32];
        out.copy_from_slice(&result);
        out
    };
    assert_eq!(secret_hash.len(), 32);
    let mut gen = rand_chacha::ChaChaRng::from_seed(secret_hash);
    let key_size = 32;
    let mut out = Vec::with_capacity(key_size);
    for _i in 0..key_size {
        out.push(gen.gen::<u8>());
    }

    out
}

fn launch_api(frcfg: &config::FrontendConfig) {
    let postgress_url =
        std::env::var("DATABASE_URL").expect("'DATABASE_URL' environment variable is not set");
    let pg_conn_manager =
        diesel::r2d2::ConnectionManager::<diesel::pg::PgConnection>::new(postgress_url);
    let pool = r2d2::Pool::new(pg_conn_manager).expect("coudln't initialize DB connection pool");
    let config = cfg::get_config();

    let cfg1 = frcfg.clone();
    let cfg2 = frcfg.clone();

    rocket::ignite()
        .manage(pool)
        .manage(config.clone())
        .attach(AdHoc::on_attach("DeriveBrancaSecretKey", move |rocket| {
            Ok(rocket.manage(cfg1.secret.clone()))
        }))
        .attach(AdHoc::on_attach("GetEnvironmentKind", move |rocket| {
            Ok(rocket.manage(cfg2.env))
        }))
        .mount(
            "/",
            routes![
                route_auth_anonymous,
                route_auth_simple,
                route_submissions_send,
                route_submissions_list,
                route_toolchains_list,
                route_users_create,
                route_api_info,
            ],
        )
        .register(catchers![catch_bad_request])
        .launch();
}

fn launch_root_login_server(logger: &slog::Logger, cfg: &config::FrontendConfig) {
    use std::sync::Arc;
    let key = derive_branca_key(&cfg.secret);
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
    let cfg = config::FrontendConfig::obtain();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, slog::o!("app"=>"jjs:frontend"));
    slog::info!(logger, "starting");

    launch_root_login_server(&logger, &cfg);
    launch_api(&cfg);
}
