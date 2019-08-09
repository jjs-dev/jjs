use super::{InvocationRequestsRepo, Repo, RunsRepo, UsersRepo};
use crate::{schema::*, Error};
use diesel::{prelude::*, r2d2::ConnectionManager};
use r2d2::{Pool, PooledConnection};

pub struct DieselRepo {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DieselRepo {
    fn conn(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
        self.pool.get().map_err(Into::into)
    }

    pub(crate) fn new(conn_url: &str) -> Result<DieselRepo, Error> {
        let conn_manager = ConnectionManager::new(conn_url);
        let pool = Pool::new(conn_manager)?;
        Ok(DieselRepo { pool })
    }
}
