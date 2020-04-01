use crate::repo::UsersRepo as _;
use anyhow::Result;
#[derive(Debug)]
pub struct DbConn {
    mem: Box<crate::repo::MemoryRepo>,
    pg: Option<Box<crate::repo::DieselRepo>>,
    redis: Option<Box<crate::repo::RedisRepo>>,
}

impl DbConn {

    fn users_repo(&self) -> &dyn crate::repo::UsersRepo {
        if let Some(pg) = &self.pg {
            return &**pg;
        }
        return &*self.mem;
    }
    fn runs_repo(&self) -> &dyn crate::repo::RunsRepo {
        if let Some(pg) = &self.pg {
            return &**pg;
        }
        return &*self.mem;
    }

    fn invocations_repo(&self) -> &dyn crate::repo::InvocationsRepo {
        if let Some(pg) = &self.pg {
            return &**pg;
        }
        return &*self.mem;
    }
}

impl DbConn {

    pub async fn user_try_load_by_login(&self, login: &str) -> Result<Option<crate::schema::User>> {
        self.users_repo().user_try_load_by_login(login).await
    }


    pub async fn load_runs_with_last_invocations(&self) -> Result<Vec<(crate::schema::Run, crate::schema::Invocation)>> {
        self.invocations_repo().load_runs_with_last_invocations().await
    }

    pub async fn run_load(&self, id: crate::schema::RunId) -> Result<crate::schema::Run> {
        self.runs_repo().run_load(id).await
    }
}
