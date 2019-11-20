use super::{InvocationRequestsRepo, Repo, RunsRepo, UsersRepo};
use crate::schema::*;
use anyhow::{Context, Result};
use diesel::{prelude::*, r2d2::ConnectionManager};
use r2d2::{Pool, PooledConnection};

pub struct DieselRepo {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl std::fmt::Debug for DieselRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("DieselRepo").finish()
    }
}

impl DieselRepo {
    fn conn(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>> {
        self.pool.get().context("db connection failed")
    }

    pub(crate) fn new(conn_url: &str) -> Result<DieselRepo> {
        let conn_manager = ConnectionManager::new(conn_url);
        let mut pool_builder = Pool::builder();
        // TODO refactor
        if let Some(timeout) = std::env::var("JJS_DB_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            let dur = std::time::Duration::from_secs(timeout);
            pool_builder = pool_builder.connection_timeout(dur);
        }
        let pool = pool_builder.build(conn_manager)?;
        Ok(DieselRepo { pool })
    }
}

mod impl_users {
    use super::*;
    use crate::schema::users::dsl::*;

    impl UsersRepo for DieselRepo {
        fn user_new(&self, user_data: NewUser) -> Result<User> {
            let user = User {
                id: uuid::Uuid::new_v4(),
                username: user_data.username,
                password_hash: user_data.password_hash,
                groups: user_data.groups,
            };
            diesel::insert_into(users)
                .values(&user)
                .execute(&self.conn()?)?;

            Ok(user)
        }

        fn user_try_load_by_login(&self, login: &str) -> Result<Option<User>> {
            Ok(users
                .filter(username.eq(login))
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
        fn inv_req_new(&self, inv_req_data: NewInvocationRequest) -> Result<InvocationRequest> {
            diesel::insert_into(invocation_requests)
                .values(&inv_req_data.to_raw()?)
                .get_result(&self.conn()?)
                .context("failed to load invocation request")
                .and_then(|raw| InvocationRequest::from_raw(&raw))
                .map_err(Into::into)
        }

        fn inv_req_pop(&self) -> Result<Option<InvocationRequest>> {
            let conn = self.conn()?;
            conn.transaction::<_, anyhow::Error, _>(|| {
                let waiting_submission = invocation_requests
                    .limit(1)
                    .load::<RawInvocationRequest>(&conn)?;
                let waiting_submission = waiting_submission.into_iter().next();
                match waiting_submission {
                    Some(s) => {
                        diesel::delete(invocation_requests)
                            .filter(id.eq(s.id))
                            .execute(&conn)?;

                        Ok(Some(InvocationRequest::from_raw(&s)?))
                    }
                    None => Ok(None),
                }
            })
            .context("failed to load invocation request")
        }
    }
}

mod impl_runs {
    use super::*;
    use crate::schema::runs::dsl::*;

    impl RunsRepo for DieselRepo {
        fn run_new(&self, run_data: NewRun) -> Result<Run> {
            diesel::insert_into(runs)
                .values(&run_data)
                .get_result(&self.conn()?)
                .map_err(Into::into)
        }

        fn run_try_load(&self, run_id: i32) -> Result<Option<Run>> {
            Ok(runs
                .filter(id.eq(run_id))
                .load::<Run>(&self.conn()?)?
                .into_iter()
                .next())
        }

        fn run_update(&self, run_id: RunId, patch: RunPatch) -> Result<()> {
            diesel::update(runs)
                .filter(id.eq(run_id))
                .set(&patch)
                .execute(&self.conn()?)
                .map(|_| ())
                .map_err(Into::into)
        }

        fn run_delete(&self, run_id: RunId) -> Result<()> {
            diesel::delete(runs)
                .filter(id.eq(run_id))
                .execute(&self.conn()?)
                .map(|_| ())
                .map_err(Into::into)
        }

        fn run_select(&self, with_run_id: Option<RunId>, limit: Option<u32>) -> Result<Vec<Run>> {
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
