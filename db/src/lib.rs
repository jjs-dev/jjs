#[macro_use]
extern crate diesel;

mod connect;
pub mod repo;
pub mod schema;

pub use connect::connect_memory;
use snafu_derive::Snafu;
use std::fmt::{self, Debug, Display, Formatter};

#[derive(Snafu, Debug)]
pub enum Error {
    Diesel {
        source: diesel::result::Error,
    },
    Other {
        source: Box<dyn std::error::Error + 'static>,
    },
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
