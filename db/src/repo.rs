mod diesel_pg;
mod memory;

pub use diesel_pg::DieselRepo;
pub use memory::MemoryRepo;

use crate::{schema::*, Error};

pub trait RunsRepo: Send + Sync {
    fn run_new(&self, run_data: NewRun) -> Result<Run, Error>;
    fn run_load(&self, run_id: RunId) -> Result<Run, Error>;
    fn run_update(&self, run_id: RunId, patch: RunPatch) -> Result<(), Error>;
    fn run_delete(&self, run_id: RunId) -> Result<(), Error>;
    fn run_select(&self, with_run_id: Option<RunId>, limit: Option<u32>)
        -> Result<Vec<Run>, Error>;
}

pub trait InvocationRequestsRepo: Send + Sync {
    fn inv_req_new(&self, inv_req_data: NewInvocationRequest) -> Result<InvocationRequest, Error>;
    fn inv_req_pop(&self) -> Result<Option<InvocationRequest>, Error>;
}

pub trait UsersRepo: Send + Sync {
    fn user_new(&self, user_data: NewUser) -> Result<User, Error>;
    fn user_try_load_by_login(&self, login: String) -> Result<Option<User>, Error>;
}

pub trait Repo: RunsRepo + InvocationRequestsRepo + UsersRepo {}