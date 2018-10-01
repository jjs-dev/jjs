#![feature(plugin)]
#![plugin(rocket_codegen)]

use rocket::Data;
use std::io;
use frontend_api::proto_capnp::*;

#[derive(Debug)]
enum Error {
    ParseError(capnp::Error),
    SchemaError(u16),
}

impl From<capnp::Error> for Error {
    fn from(e: capnp::Error) -> Self {
        Error::ParseError(e)
    }
}

impl From<capnp::NotInSchema> for Error {
    fn from(e: capnp::NotInSchema) -> Self {
        Error::SchemaError(e.0)
    }
}

impl<'a> rocket::response::Responder<'a> for Error {
    fn respond_to<'b>(self, _: &rocket::Request) -> rocket::response::Result<'a> {
        rocket::Response::build()
            .status(rocket::http::Status::BadRequest)
            .sized_body(io::Cursor::new(format!("Parse error: {:#?}", &self)))
            .ok()
    }
}

fn route_ping(q: &ping_request::Reader, out: &mut ping_result::Builder) -> Result<(), Error> {
    out.reborrow().init_ok().set_data(q.get_data()?);
    Ok(())
}

#[post("/api", data = "<raw_query>")]
fn route_api(raw_query: Data) -> Result<String, Error> {
    let stream = raw_query.open();
    let mut stream_reader = io::BufReader::new(stream);
    let query_reader = capnp::serialize_packed::read_message(
        &mut stream_reader, capnp::message::ReaderOptions::new()).unwrap();
    let query = query_reader.get_root::<request::Reader>()?;
    let body = query.get_query()?.which();
   /* let body = match body {
        Ok(x) => x,
        Err(capnp::NotInSchema(bad_discr)) => {
            return Err(Error::SchemaError(bad_discr));
        }
    };*/
    let body = body?;

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

    //Ok("".to_string())
}


fn main() {
    rocket::ignite().mount("/", routes![route_api]).launch();
}
