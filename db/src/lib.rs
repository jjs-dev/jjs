#[macro_use]
extern crate diesel;

pub mod connect;
pub mod repo;
pub mod schema;

pub use connect::connect_env;
pub use repo::Repo as DbConn;
use snafu_derive::Snafu;
use std::fmt::{self, Debug, Display, Formatter};

#[derive(Snafu, Debug)]
pub enum Error {
    R2d2 {
        source: r2d2::Error,
    },
    Diesel {
        source: diesel::result::Error,
    },
    Other {
        source: Box<dyn std::error::Error + 'static>,
    },
}

impl From<r2d2::Error> for Error {
    fn from(source: r2d2::Error) -> Error {
        Error::R2d2 { source }
    }
}

impl From<diesel::result::Error> for Error {
    fn from(source: diesel::result::Error) -> Error {
        Error::Diesel { source }
    }
}

struct StringError(&'static str);

impl Display for StringError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self.0, f)
    }
}

impl Debug for StringError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(self.0, f)
    }
}

impl std::error::Error for StringError {}

impl Error {
    fn string(s: &'static str) -> Self {
        Error::Other {
            source: Box::new(StringError(s)),
        }
    }
}
