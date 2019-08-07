#[macro_use]
extern crate diesel;

pub mod schema;
pub mod repo;

use snafu_derive::Snafu;
use std::fmt::{self, Formatter, Display, Debug};

#[derive(Snafu, Debug)]
pub enum Error {
    Diesel {
        source: diesel::result::Error,
    },
    Other {
        source: Box<dyn std::error::Error + 'static>
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
            source: Box::new(StringError(s))
        }
    }
}