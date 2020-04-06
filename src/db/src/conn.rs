use anyhow::{Context as _, Result};

#[derive(Debug)]
pub struct DbConn {
    pub(crate) mem: crate::repo::MemoryRepo,
    pub(crate) pg: Option<crate::repo::DieselRepo>,
    pub(crate) redis: Option<crate::repo::RedisRepo>,
}

impl DbConn {
    fn users_repo(&self) -> &dyn crate::repo::UsersRepo {
        if let Some(pg) = &self.pg {
            return &*pg;
        }
        &self.mem
    }

    fn runs_repo(&self) -> &dyn crate::repo::RunsRepo {
        if let Some(pg) = &self.pg {
            return &*pg;
        }
        &self.mem
    }

    fn invocations_repo(&self) -> &dyn crate::repo::InvocationsRepo {
        if let Some(pg) = &self.pg {
            return &*pg;
        }
        &self.mem
    }

    fn kv_repo(&self) -> &dyn crate::repo::KvRepo {
        if let Some(redis) = &self.redis {
            return &*redis;
        }
        if let Some(pg) = &self.pg {
            return &*pg;
        }
        &self.mem
    }

    fn participations_repo(&self) -> &dyn crate::repo::ParticipationsRepo {
        if let Some(pg) = &self.pg {
            return &*pg;
        }
        &self.mem
    }
}

impl DbConn {
    pub async fn user_try_load_by_login(&self, login: &str) -> Result<Option<crate::schema::User>> {
        self.users_repo().user_try_load_by_login(login).await
    }

    pub async fn load_runs_with_last_invocations(
        &self,
    ) -> Result<Vec<(crate::schema::Run, crate::schema::Invocation)>> {
        self.invocations_repo()
            .load_runs_with_last_invocations()
            .await
    }

    pub async fn run_load(&self, id: crate::schema::RunId) -> Result<crate::schema::Run> {
        self.runs_repo().run_load(id).await
    }

    pub async fn inv_find_waiting(
        &self,
        offset: u32,
        count: u32,
        predicate: &mut (dyn FnMut(crate::schema::Invocation) -> Result<bool> + Send + Sync),
    ) -> Result<Vec<crate::schema::Invocation>> {
        self.invocations_repo()
            .inv_find_waiting(offset, count, predicate)
            .await
    }

    pub async fn inv_last(
        &self,
        run_id: crate::schema::RunId,
    ) -> Result<crate::schema::Invocation> {
        self.invocations_repo().inv_last(run_id).await
    }

    pub async fn run_select(
        &self,
        with_run_id: Option<crate::schema::RunId>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::schema::Run>> {
        self.runs_repo().run_select(with_run_id, limit).await
    }

    pub async fn run_try_load(
        &self,
        run_id: crate::schema::RunId,
    ) -> Result<Option<crate::schema::Run>> {
        self.runs_repo().run_try_load(run_id).await
    }

    pub async fn run_new(&self, run_data: crate::schema::NewRun) -> Result<crate::schema::Run> {
        self.runs_repo().run_new(run_data).await
    }

    pub async fn inv_new(
        &self,
        inv_req_data: crate::schema::NewInvocation,
    ) -> Result<crate::schema::Invocation> {
        self.invocations_repo().inv_new(inv_req_data).await
    }

    pub async fn run_update(
        &self,
        run_id: crate::schema::RunId,
        patch: crate::schema::RunPatch,
    ) -> Result<()> {
        self.runs_repo().run_update(run_id, patch).await
    }

    pub async fn inv_update(
        &self,
        inv_id: crate::schema::InvocationId,
        patch: crate::schema::InvocationPatch,
    ) -> Result<()> {
        self.invocations_repo().inv_update(inv_id, patch).await
    }

    pub async fn run_delete(&self, run_id: crate::schema::RunId) -> Result<()> {
        self.runs_repo().run_delete(run_id).await
    }

    pub async fn user_new(&self, user_data: crate::schema::NewUser) -> Result<crate::schema::User> {
        self.users_repo().user_new(user_data).await
    }

    pub async fn inv_add_outcome_header(
        &self,
        inv_id: crate::schema::InvocationId,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> Result<()> {
        self.invocations_repo()
            .inv_add_outcome_header(inv_id, header)
            .await
    }

    pub async fn kv_get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let maybe_raw_data = self
            .kv_repo()
            .kv_get_raw(key)
            .await
            .context("failed to load value")?;
        match maybe_raw_data {
            Some(raw_data) => serde_json::from_slice(&raw_data)
                .context("parse error")
                .map(Some),
            None => Ok(None),
        }
    }

    pub async fn kv_put<T: serde::ser::Serialize>(&self, key: &str, value: T) -> Result<()> {
        let raw_data = serde_json::to_vec(&value).context("serialize error")?;
        self.kv_repo().kv_put_raw(key, &raw_data).await
    }

    pub async fn kv_del(&self, key: &str) -> Result<()> {
        self.kv_repo().kv_del(key).await
    }

    pub async fn part_lookup(
        &self,
        user_id: crate::schema::UserId,
        contest_id: &str,
    ) -> Result<Option<crate::schema::Participation>> {
        self.participations_repo()
            .part_lookup(user_id, contest_id)
            .await
    }

    pub async fn part_new(
        &self,
        data: crate::schema::NewParticipation,
    ) -> Result<crate::schema::Participation> {
        self.participations_repo().part_new(data).await
    }
}
