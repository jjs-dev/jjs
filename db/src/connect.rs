use crate::{
    repo::{DieselRepo, MemoryRepo, Repo},
    Error,
};
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

pub fn connect(options: ConnectOptions) -> Result<Box<dyn Repo>, Error> {
    if let Some(pg_conn_str) = options.pg {
        Ok(Box::new(DieselRepo::new(&pg_conn_str)?))
    } else {
        Ok(Box::new(MemoryRepo::new()))
    }
}

pub fn connect_env() -> Result<Box<dyn Repo>, Error> {
    let opts = ConnectOptions {
        pg: env::var("DATABASE_URL").ok(),
    };
    opts.warn();
    connect(opts)
}
