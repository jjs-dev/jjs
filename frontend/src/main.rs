#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate failure_derive;
extern crate frontend_api;

use rocket::Data;
use std::io;
//use std::io::Read;
use std::string::FromUtf8Error;

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

impl From<FromUtf8Error> for TransportError {
    fn from(e: FromUtf8Error) -> Self {
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
            .sized_body(io::Cursor::new(format!("Error: {:#?}", &self)))
            .ok()
    }
}

type ApiResult<Res> = Result<Res, TransportError>;

fn api_fun_ping(q: &frontend_api::PingRequest) -> ApiResult<frontend_api::PingResult> {
    Ok(Ok(frontend_api::PingSuccess {
        data: q.data.clone(),
    }))
}

#[post("/api", data = "<raw_query>")]
fn route_api(raw_query: Data) -> ApiResult<String> {
    let mut stream = raw_query.open();

    let mut query = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut query)?;
    //stream.read_to_end(&mut query)?;

    let query = String::from_utf8(query)?;

    let query: frontend_api::Request = serde_json::from_str(&query)?;

    let response_body = match query.query {
        frontend_api::RequestBody::Ping(ref ping) => {
            frontend_api::ResponseBody::Ping(api_fun_ping(ping)?)
        }
    };

    let response = frontend_api::Response {
        result: response_body,
    };

    let response = serde_json::to_string(&response)?;



    Ok(response)

    /*
        let mut response_buidler = capnp::message::Builder::new_default();
        let response = response_buidler.init_root::<response::Builder>().init_result();

        match body {
            request_body::Ping(x) => {
                let x = x?;
                println!("got ping: {:#?}", x.reborrow().get_data()?);
                route_ping(&x, &mut response.init_ping());
                //Ok("".to_string())
            }
            request_body::UnusedFieldBecauseUnionMustHaveAtLeastTwoMembers(_) => unreachable!()
        };

        let mut out_buf = Vec::new();
        capnp::serialize_packed::write_message(&mut out_buf, &response_buidler).unwrap();

        Ok(String::from_utf8_lossy(&out_buf).to_string())
    */
}


fn main() {
    rocket::ignite().mount("/", routes![route_api]).launch();
}
