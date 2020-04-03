#[macro_use]
extern crate diesel;

mod conn;
pub mod connect;
pub mod repo;
pub mod schema;

pub use conn::DbConn;
pub use connect::connect_env;

pub use anyhow::Error;
pub mod prelude {
    pub use crate::repo::{KvRepo as _, Repo as _, RunsRepo as _, UsersRepo as _};
}
