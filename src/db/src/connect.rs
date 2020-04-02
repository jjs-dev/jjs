use crate::{
    repo::{DieselRepo, MemoryRepo},
    DbConn,
};
use anyhow::{Context, Result};
use std::env;

pub struct ConnectOptions {
    /// Postgres connection string
    pg: Option<String>,
}

impl ConnectOptions {
    fn warn(&self) {
        if cfg!(not(test)) && self.pg.is_none() {
            eprintln!(
                "warning: pg url not provided in DATABASE_URL. \
                 JJS is unusable in such configuration."
            );
        }
    }
}

pub fn connect(options: ConnectOptions) -> Result<DbConn> {
    let mem = MemoryRepo::new();
    let pg = match options.pg {
        Some(pg_conn_str) => {
            Some((DieselRepo::new(&pg_conn_str)).context("cannot connect to postgres")?)
        }
        None => None,
    };
    let redis = None;
    Ok(DbConn { mem, pg, redis })
}

pub fn connect_env() -> Result<crate::DbConn> {
    let opts = ConnectOptions {
        pg: env::var("DATABASE_URL").ok(),
    };
    opts.warn();
    connect(opts)
}

pub fn connect_memory() -> Result<crate::DbConn> {
    let opts = ConnectOptions { pg: None };
    connect(opts)
}
