#![feature(box_syntax)]

use std::env::var;

pub fn connect_pg() -> db::Db {
    let pg_url = var("POSTGRES_URL").unwrap();
    let conn = postgres::Connection::connect(pg_url, postgres::TlsMode::None).unwrap();
    db::Db {
        submissions: box db::submission::PgSubmissions::new(box conn),
    }
}

pub fn connect_mq() -> amqp::Session {
    amqp::Session::open_url("amqp://localhost//").unwrap()
}
