#![feature(box_syntax)]

use std::env::var;

pub fn connect_pg() -> db::Db {
    let pg_url = var("POSTGRES_URL").expect("POSTGRES_URL not set");
    let conn = postgres::Connection::connect(pg_url, postgres::TlsMode::None)
        .expect("couldn't connect to postgres");
    db::Db {
        submissions: db::submission::Submissions::new(box conn),
    }
}
