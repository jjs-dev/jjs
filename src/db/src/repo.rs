mod memory;
mod pg;
mod redis;

pub use self::redis::RedisRepo;
pub use memory::MemoryRepo;
pub use pg::PgRepo;

use crate::schema::*;
use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::stream::{StreamExt as _, TryStreamExt as _};

#[async_trait]
pub trait RunsRepo: std::fmt::Debug + Send + Sync {
    async fn run_new(&self, run_data: NewRun) -> Result<Run>;
    async fn run_try_load(&self, run_id: RunId) -> Result<Option<Run>>;
    async fn run_load(&self, run_id: RunId) -> Result<Run> {
        match self.run_try_load(run_id).await? {
            Some(run) => Ok(run),
            None => bail!("run_load: unknown run_id"),
        }
    }
    async fn run_update(&self, run_id: RunId, patch: RunPatch) -> Result<()>;
    async fn run_delete(&self, run_id: RunId) -> Result<()>;
    async fn run_select(&self, with_run_id: Option<RunId>, limit: Option<u32>) -> Result<Vec<Run>>;
}

#[async_trait]
pub trait InvocationsRepo: RunsRepo + Send + Sync {
    async fn inv_new(&self, inv_req_data: NewInvocation) -> Result<Invocation>;

    async fn inv_last(&self, run_id: RunId) -> Result<Invocation>;

    async fn inv_find_waiting(
        &self,
        offset: u32,
        count: u32,
        predicate: &mut (dyn FnMut(Invocation) -> Result<bool> + Send + Sync),
    ) -> Result<Vec<Invocation>>;

    async fn load_runs_with_last_invocations(&self) -> Result<Vec<(Run, Invocation)>> {
        let runs = self.run_select(None, None).await?.into_iter();
        let runs = futures::stream::iter(runs);
        runs.then(|r| async {
            let r_id = r.id;
            (r, self.inv_last(r_id).await)
        })
        .map(|(run, maybe_invocation)| match maybe_invocation {
            Ok(inv) => Ok((run, inv)),
            Err(err) => Err(err),
        })
        .try_collect()
        .await
    }

    async fn inv_add_outcome_header(
        &self,
        inv_id: InvocationId,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> Result<()>;

    async fn inv_update(&self, inv_id: InvocationId, patch: InvocationPatch) -> Result<()>;
}

#[async_trait]
pub trait UsersRepo: Send + Sync {
    async fn user_new(&self, user_data: NewUser) -> Result<User>;
    async fn user_try_load_by_login(&self, login: &str) -> Result<Option<User>>;
}

#[async_trait]
pub trait KvRepo: Send + Sync {
    async fn kv_put_raw(&self, key: &str, value: &[u8]) -> Result<()>;

    async fn kv_get_raw(&self, key: &str) -> Result<Option<Vec<u8>>>;

    async fn kv_del(&self, key: &str) -> Result<()>;
}

#[async_trait]
pub trait ParticipationsRepo: Send + Sync {
    async fn part_new(&self, part_data: NewParticipation) -> Result<Participation>;
    async fn part_find(&self, id: ParticipationId) -> Result<Option<Participation>>;
    async fn part_lookup(&self, user_id: UserId, contest_id: &str)
    -> Result<Option<Participation>>;
}
pub trait Repo: RunsRepo + InvocationsRepo + UsersRepo + KvRepo + ParticipationsRepo {}

impl dyn Repo {}
