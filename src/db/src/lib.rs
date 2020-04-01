#[macro_use]
extern crate diesel;

mod conn;
pub mod connect;
pub mod repo;
pub mod schema;

pub use connect::connect_env;
pub use conn::DbConn;

pub use anyhow::Error;
pub mod prelude {
    pub use crate::repo::{Repo as _, KvRepo as _, UsersRepo as _, RunsRepo as _};
}