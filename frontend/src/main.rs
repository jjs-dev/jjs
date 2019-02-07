#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
use rocket::{http::Status, State};
use rocket_contrib::json::Json;
//use rocket::response::Responder;
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
        //res.respond_to(request)
    }
}

type Response<R> = Result<Json<R>, FrontendError>;

type DbPool = r2d2::Pool<r2d2_postgres::PostgresConnectionManager>;

#[post("/auth/anonymous")]
fn route_auth_anonymous() -> Response<Result<frontend_api::AuthToken, frontend_api::CommonError>> {
    let res =Ok(frontend_api::AuthToken { buf: "".to_string() });

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
) -> Response<Result<frontend_api::SubmissionId, frontend_api::SubmitError>> {
    use std::ops::Deref;
    let conn = db.get().expect("couldn't connect to DB");
    let db = db::Db::new(conn.deref());
    let res = db.submissions.create_submission(data.toolchain);
    let res = Ok(res.id);
    Ok(Json(res))
}

#[get("/toolchains/list")]
fn route_toolchains_list()  -> Response<Result<Vec<frontend_api::ToolchainInformation>, frontend_api::CommonError>> {
    let res = vec![frontend_api::ToolchainInformation {
        name: "cpp".to_string(),
        id: 0
    }];
    let res = Ok(res);

    Ok(Json(res))
}

fn main() {
    dotenv::dotenv().ok();
    let port = 1779;
    let listen_address = format!("127.0.0.1:{}", port);
    let postgress_url =
        std::env::var("POSTGRES_URL").expect("'POSTGRES_URL' environment variable is not set");
    let pg_conn_manager =
        r2d2_postgres::PostgresConnectionManager::new(postgress_url, r2d2_postgres::TlsMode::None)
            .expect("coudln't initialize DB connection pool");
    let pool = r2d2::Pool::new(pg_conn_manager).expect("coudln't initialize DB connection pool");
    println!("JJS api frontend is listening on {}", &listen_address);
    rocket::ignite()
        .manage(pool)
        .mount(
            "/",
            routes![
                route_ping,
                route_auth_anonymous,
                route_auth_simple,
                route_submissions_send
            ],
        )
        .launch();
}
