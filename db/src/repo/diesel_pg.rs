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

mod impl_users {
    use super::*;
    use crate::schema::users::dsl::*;

    impl UsersRepo for DieselRepo {
        fn user_new(&self, user_data: NewUser) -> Result<User, Error> {
            diesel::insert_into(users)
                .values(&user_data)
                .get_result(&self.conn()?)
                .map_err(Into::into)
        }

        fn user_try_load_by_login(&self, login: String) -> Result<Option<User>, Error> {
            Ok(users
                .filter(username.eq(&login))
                .load(&self.conn()?)?
                .into_iter()
                .next())
        }
    }
}

mod impl_inv_reqs {
    use super::*;
    use crate::schema::invocation_requests::dsl::*;

    impl InvocationRequestsRepo for DieselRepo {
        fn inv_req_new(
            &self,
            inv_req_data: NewInvocationRequest,
        ) -> Result<InvocationRequest, Error> {
            diesel::insert_into(invocation_requests)
                .values(&inv_req_data)
                .get_result(&self.conn()?)
                .map_err(Into::into)
        }

        fn inv_req_pop(&self) -> Result<Option<InvocationRequest>, Error> {
            let conn = self.conn()?;
            conn.transaction::<_, diesel::result::Error, _>(|| {
                let waiting_submission = invocation_requests
                    .limit(1)
                    .load::<InvocationRequest>(&conn)?;
                let waiting_submission = waiting_submission.into_iter().next();
                match waiting_submission {
                    Some(s) => {
                        diesel::delete(invocation_requests)
                            .filter(id.eq(s.id))
                            .execute(&conn)?;

                        Ok(Some(s))
                    }
                    None => Ok(None),
                }
            })
            .map_err(Into::into)
        }
    }
}

mod impl_runs {
    use super::*;
    use crate::schema::runs::dsl::*;

    impl RunsRepo for DieselRepo {
        fn run_new(&self, run_data: NewRun) -> Result<Run, Error> {
            diesel::insert_into(runs)
                .values(&run_data)
                .get_result(&self.conn()?)
                .map_err(Into::into)
        }

        fn run_load(&self, run_id: RunId) -> Result<Run, Error> {
            let maybe_run = runs
                .filter(id.eq(run_id))
                .load::<Run>(&self.conn()?)?
                .into_iter()
                .next();
            match maybe_run {
                Some(r) => Ok(r),
                None => Err(Error::string("run_load@diesel_pg: unknown run id")),
            }
        }

        fn run_update(&self, run_id: RunId, patch: RunPatch) -> Result<(), Error> {
            diesel::update(runs)
                .filter(id.eq(run_id))
                .set(&patch)
                .execute(&self.conn()?)
                .map(|_| ())
                .map_err(Into::into)
        }

        fn run_delete(&self, run_id: RunId) -> Result<(), Error> {
            diesel::delete(runs)
                .filter(id.eq(run_id))
                .execute(&self.conn()?)
                .map(|_| ())
                .map_err(Into::into)
        }

        fn run_select(
            &self,
            with_run_id: Option<RunId>,
            limit: Option<u32>,
        ) -> Result<Vec<Run>, Error> {
            let mut query = runs.into_boxed();

            if let Some(rid) = with_run_id {
                query = query.filter(id.eq(rid));
            }
            let limit = limit.map(i64::from).unwrap_or(i64::max_value());
            Ok(query.limit(limit).load(&self.conn()?)?)
        }
    }
}

impl Repo for DieselRepo {}
