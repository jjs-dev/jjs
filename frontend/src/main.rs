#![feature(proc_macro_hygiene, decl_macro, type_alias_enum_variants, box_syntax)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde_derive;

mod security;

use cfg::Config;
use db::schema::{NewSubmission, Submission, SubmissionState};
use diesel::prelude::*;
use rocket::{fairing::AdHoc, http::Status, State};
use rocket_contrib::json::Json;
use security::{SecretKey, Token};
use std::fmt::Debug;

#[get("/ping")]
fn route_ping() -> &'static str {
    "\"pong\""
}

#[derive(Debug)]
enum FrontendError {
    Internal(Option<Box<dyn Debug>>),
    Db(diesel::result::Error),
}

impl<'r> rocket::response::Responder<'r> for FrontendError {
    fn respond_to(self, _request: &rocket::Request) -> rocket::response::Result<'r> {
        eprintln!("FrontendError: {:?}", &self);
        let res = match self {
            FrontendError::Internal(_) | FrontendError::Db(_) => Status::InternalServerError,
        };
        Err(res)
    }
}

impl From<diesel::result::Error> for FrontendError {
    fn from(e: diesel::result::Error) -> Self {
        FrontendError::Db(e)
    }
}

type Response<R> = Result<Json<R>, FrontendError>;

type DbPool = r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>;

#[post("/auth/anonymous")]
fn route_auth_anonymous() -> Response<Result<frontend_api::AuthToken, frontend_api::CommonError>> {
    let res = Ok(frontend_api::AuthToken {
        buf: "".to_string(),
    });

    Ok(Json(res))
}

#[post("/auth/simple", data = "<data>")]
fn route_auth_simple(
    data: Json<frontend_api::SimpleAuthParams>,
    secret_key: State<SecretKey>,
) -> Response<Result<frontend_api::AuthToken, frontend_api::SimpleAuthError>> {
    let succ = data.login == data.password;
    let res = if succ {
        let tok = Token::create_for_user(data.login.clone());
        Ok(frontend_api::AuthToken {
            buf: tok.serialize(&secret_key.0),
        })
    } else {
        Err(frontend_api::SimpleAuthError::IncorrectPassword)
    };

    Ok(Json(res))
}

#[post("/submissions/send", data = "<data>")]
fn route_submissions_send(
    data: Json<frontend_api::SubmitDeclaration>,
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
        .map_err(|e| FrontendError::Internal(Some(box e)))?;
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

#[post("/submissions/list", data = "<limit>")]
fn route_submissions_list(
    limit: Json<u32>,
    db: State<DbPool>,
    _token: Token,
) -> Response<Result<Vec<frontend_api::SubmissionInformation>, frontend_api::CommonError>> {
    use db::schema::submissions::dsl::*;
    let conn = db.get().expect("Couldn't connect to DB");
    let user_submissions = submissions
        .limit(i64::from(limit))
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

fn main() {
    dotenv::dotenv().ok();
    //let listen_address = format!("127.0.0.1:{}", port);
    let postgress_url =
        std::env::var("DATABASE_URL").expect("'DATABASE_URL' environment variable is not set");
    let pg_conn_manager =
        diesel::r2d2::ConnectionManager::<diesel::pg::PgConnection>::new(postgress_url);
    let pool = r2d2::Pool::new(pg_conn_manager).expect("coudln't initialize DB connection pool");
    let config = cfg::get_config();
    //println!("JJS api frontend is listening on {}", &listen_address);
    rocket::ignite()
        .manage(pool)
        .manage(config.clone())
        .attach(AdHoc::on_attach("DeriveBrancaSecretKey",|rocket| {
            let secret_key = rocket.config().get_string("jjs_secret_key").unwrap_or_else(|_|{
                let is_dev = rocket.config().environment.is_dev();
                if !is_dev {
                    eprintln!("Warning: couldn't obtain jjs_secret_key from configuration, providing hardcoded");
                }
                "HARDCODED_DEV_ONLY_KEY".to_string() //TODO: panic in production
            });
            let branca_key = derive_branca_key(&secret_key);
            Ok(rocket.manage(SecretKey(branca_key)))
        }))
        .mount(
            "/",
            routes![
                route_ping,
                route_auth_anonymous,
                route_auth_simple,
                route_submissions_send,
                route_submissions_list,
                route_toolchains_list,
                route_api_info,
            ],
        )
        .launch();
}
