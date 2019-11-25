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

pub trait InvocationRequestsRepo: Send + Sync {
    fn inv_req_new(&self, inv_req_data: NewInvocationRequest) -> Result<InvocationRequest>;
    fn inv_req_pop(&self) -> Result<Option<InvocationRequest>>;
}

pub trait UsersRepo: Send + Sync {
    fn user_new(&self, user_data: NewUser) -> Result<User>;
    fn user_try_load_by_login(&self, login: &str) -> Result<Option<User>>;
}

pub trait Repo: RunsRepo + InvocationRequestsRepo + UsersRepo {}
