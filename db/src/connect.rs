use crate::{
    repo::{DieselRepo, MemoryRepo, Repo},
    Error,
};

pub struct ConnectOptions {
    /// Postgres connection string
    pg: Option<String>,
}

pub fn connect(options: ConnectOptions) -> Result<Box<dyn Repo>, Error> {
    if let Some(pg_conn_str) = options.pg {
        Ok(Box::new(DieselRepo::new(&pg_conn_str)?))
    } else {
        Ok(Box::new(MemoryRepo::new()))
    }
}
