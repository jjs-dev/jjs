#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate failure_derive;
extern crate frontend_api;

mod security;
mod submission;

use rocket::Data;
use std::{io, time::Duration};

#[derive(Fail, Debug)]
enum TransportError {
    #[fail(display = "Error occured while parsing: {:?}", _0)]
    Parse(#[cause] serde_json::Error),
    #[fail(display = "Couldn't decode request body: {:?}", _0)]
    Decode(#[cause] std::string::FromUtf8Error),
    #[fail(display = "IO error: {:?}", _0)]
    Io(#[cause] io::Error),

}

impl From<serde_json::Error> for TransportError {
    fn from(e: serde_json::Error) -> Self {
        TransportError::Parse(e)
    }
}

impl From<std::string::FromUtf8Error> for TransportError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        TransportError::Decode(e)
    }
}

impl From<io::Error> for TransportError {
    fn from(e: io::Error) -> Self {
        TransportError::Io(e)
    }
}

impl<'a> rocket::response::Responder<'a> for TransportError {
    fn respond_to<'b>(self, _: &rocket::Request) -> rocket::response::Result<'a> {
        rocket::Response::build()
            .status(rocket::http::Status::BadRequest)
            .sized_body(io::Cursor::new(format!("{:#?}", &self)))
            .ok()
    }
}

type ApiResult<Res> = Result<Res, TransportError>;


pub(crate) struct ApiFunContext<'a> {
    db: &'a mut db::Db,
}


fn api_fun_ping(q: &frontend_api::util::PingRequest) -> ApiResult<frontend_api::util::PingResult> {
    Ok(Ok(frontend_api::util::PingSuccess {
        data: q.data.clone(),
    }))
}

fn api_fun_passwd_auth(q: &frontend_api::auth::PasswordAuthRequest) -> ApiResult<frontend_api::auth::PasswordAuthResult> {
    //TODO check password
    let token = security::Token::create_for_user(q.login.clone(), Duration::from_secs(3600));
    let succ = frontend_api::auth::PasswordAuthSuccess {
        new_token: token.key,
    };
    Ok(Ok(succ))
}

fn api_fun_s8n<'a>(q: &frontend_api::SubmissionResult, ctx: ApiFunContext<'a>) -> ApiResult<&frontend_api::SubmissionResult> {
    unimplemented!()
}

#[post("/api", data = "<raw_query>")]
fn route_api(raw_query: Data) -> ApiResult<String> {
    let mut stream = raw_query.open();

    let mut query = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut query)?;

    let query = String::from_utf8(query)?;

    let query: frontend_api::Request = serde_json::from_str(&query)?;

    let response_body = match query.query {
        frontend_api::RequestBody::Ping(ref ping) =>
            frontend_api::ResponseBody::Ping(api_fun_ping(ping)?),

        frontend_api::RequestBody::PasswordAuth(ref q) =>
            frontend_api::ResponseBody::PasswordAuth(api_fun_passwd_auth(q)?),
        _ => unimplemented!()
    };

    let response = frontend_api::Response {
        result: response_body,
    };

    let response = serde_json::to_string(&response)?;


    Ok(response)
}


fn main() {
    rocket::ignite().mount("/", routes![route_api]).launch();
}
