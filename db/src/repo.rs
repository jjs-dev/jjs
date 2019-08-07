mod memory;

use crate::{schema::*, Error};

pub trait RunsRepo {
    fn run_new(&self, run_data: NewRun) -> Result<Run, Error>;
    fn run_load(&self, run_id: RunId) -> Result<Run, Error>;
    fn run_update(&self, run_id: RunId, patch: RunPatch) -> Result<(), Error>;
}

pub trait InvocationRequestsRepo {
    fn inv_req_new(&self, inv_req_data: NewInvocationRequest) -> Result<InvocationRequest, Error>;
    fn inv_req_pop(&self) -> Result<Option<InvocationRequest>, Error>;
}

pub trait UsersRepo {
    fn user_new(&self, user_data: NewUser) -> Result<User, Error>;
}

pub trait Repo: RunsRepo + InvocationRequestsRepo {}