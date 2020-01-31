mod diesel_pg;
mod memory;

pub use diesel_pg::DieselRepo;
pub use memory::MemoryRepo;

use crate::schema::*;
use anyhow::{bail, Result};

pub trait RunsRepo: std::fmt::Debug + Send + Sync {
    fn run_new(&self, run_data: NewRun) -> Result<Run>;
    fn run_try_load(&self, run_id: RunId) -> Result<Option<Run>>;
    fn run_load(&self, run_id: RunId) -> Result<Run> {
        match self.run_try_load(run_id)? {
            Some(run) => Ok(run),
            None => bail!("run_load: unknown run_id"),
        }
    }
    fn run_update(&self, run_id: RunId, patch: RunPatch) -> Result<()>;
    fn run_delete(&self, run_id: RunId) -> Result<()>;
    fn run_select(&self, with_run_id: Option<RunId>, limit: Option<u32>) -> Result<Vec<Run>>;
}

pub trait InvocationsRepo: RunsRepo + Send + Sync {
    fn inv_new(&self, inv_req_data: NewInvocation) -> Result<Invocation>;

    fn inv_last(&self, run_id: RunId) -> Result<Invocation>;

    fn inv_find_waiting(
        &self,
        offset: u32,
        count: u32,
        predicate: &mut dyn FnMut(Invocation) -> Result<bool>,
    ) -> Result<Vec<Invocation>>;

    fn load_runs_with_last_invocations(&self) -> Result<Vec<(Run, Invocation)>> {
        let runs = self.run_select(None, None)?;
        runs.into_iter()
            .map(|r| {
                let r_id = r.id;
                (r, self.inv_last(r_id))
            })
            .map(|(run, maybe_invocation)| match maybe_invocation {
                Ok(inv) => Ok((run, inv)),
                Err(err) => Err(err),
            })
            .collect()
    }

    fn inv_update(&self, inv_id: InvocationId, patch: InvocationPatch) -> Result<()>;
}

pub trait UsersRepo: Send + Sync {
    fn user_new(&self, user_data: NewUser) -> Result<User>;
    fn user_try_load_by_login(&self, login: &str) -> Result<Option<User>>;
}

pub trait Repo: RunsRepo + InvocationsRepo + UsersRepo {}
