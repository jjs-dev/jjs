use crate::{
    repo::{MemoryRepo, Repo},
    Error,
};

pub fn connect_memory() -> Result<Box<dyn Repo>, Error> {
    Ok(Box::new(MemoryRepo::new()))
}
