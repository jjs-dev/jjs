#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use cfg::Config;
use rocket::{http::Status, State};
use rocket_contrib::json::Json;

#[get("/ping")]
fn route_ping() -> &'static str {
    "JJS frontend"
}

#[derive(Debug)]
enum FrontendError {
    Internal,
}

impl<'r> rocket::response::Responder<'r> for FrontendError {
    fn respond_to(self, _request: &rocket::Request) -> rocket::response::Result<'r> {
        let res = match self {
            FrontendError::Internal => Status::InternalServerError,
        };
        Err(res)
    }
}

type Response<R> = Result<Json<R>, FrontendError>;

type DbPool = r2d2::Pool<r2d2_postgres::PostgresConnectionManager>;

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
) -> Response<Result<frontend_api::AuthToken, frontend_api::SimpleAuthError>> {
    let succ = data.login == data.password;
    let res = if succ {
        Ok(frontend_api::AuthToken {
            buf: data.login.clone(),
        })
    } else {
        Err(frontend_api::SimpleAuthError::IncorrectPassword)
    };

    Ok(Json(res))
}

#[post("/submission/send", data = "<data>")]
fn route_submissions_send(
    data: Json<frontend_api::SubmitDeclaration>,
    db: State<DbPool>,
    cfg: State<Config>,
) -> Response<Result<frontend_api::SubmissionId, frontend_api::SubmitError>> {
    use std::ops::Deref;
    let toolchain = cfg.toolchains.get(data.toolchain as usize);
    let toolchain = match toolchain {
        Some(tc) => tc.clone(),
        None => {
            let res = Err(frontend_api::SubmitError::UnknownToolchain);
            return Ok(Json(res));
        }
    };
    let conn = db.get().expect("couldn't connect to DB");
    let db = db::Db::new(conn.deref());
    let res = db.submissions.create_submission(toolchain.name);
    let res = Ok(res.id);
    Ok(Json(res))
}

#[get("/submissions/list?<limit>")]
fn route_submissions_list(
    limit: u32,
    db: State<DbPool>,
) -> Response<Result<Vec<frontend_api::SubmissionInformation>, frontend_api::CommonError>> {
    let conn = db.get().expect("Couldn't connect to DB");
    let submissions = db::Db::new(&*conn).submissions.get_all(limit);
    let submissions = submissions.iter().map(|s| frontend_api::SubmissionInformation{
        id: s.id,
        toolchain_name: s.toolchain.clone()
    }).collect();
    let res = Ok(submissions);
    Ok(Json(res))
}

#[get("/toolchains/list")]
fn route_toolchains_list(
    cfg: State<Config>,
) -> Response<Result<Vec<frontend_api::ToolchainInformation>, frontend_api::CommonError>> {
    //let res = vec![frontend_api::ToolchainInformation {
    //    name: "cpp".to_string(),
    //    id: 0,
    //}];
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

fn main() {
    dotenv::dotenv().ok();
    //let listen_address = format!("127.0.0.1:{}", port);
    let postgress_url =
        std::env::var("POSTGRES_URL").expect("'POSTGRES_URL' environment variable is not set");
    let pg_conn_manager =
        r2d2_postgres::PostgresConnectionManager::new(postgress_url, r2d2_postgres::TlsMode::None)
            .expect("coudln't initialize DB connection pool");
    let pool = r2d2::Pool::new(pg_conn_manager).expect("coudln't initialize DB connection pool");
    let config = cfg::get_config();
    //println!("JJS api frontend is listening on {}", &listen_address);
    rocket::ignite()
        .manage(pool)
        .manage(config.clone())
        .mount(
            "/",
            routes![
                route_ping,
                route_auth_anonymous,
                route_auth_simple,
                route_submissions_send,
                route_submissions_list,
                route_toolchains_list,
            ],
        )
        .launch();
}
