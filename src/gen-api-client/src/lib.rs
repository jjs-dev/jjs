#![allow(dead_code)]
#![allow(clippy::all)]
#![allow(unused_mut, unreachable_code)]
#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate url;

pub mod apis;
pub mod models;
