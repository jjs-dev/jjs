//! Implements pretty-printing of invocation request
use judging_apis::invoke::InvokeRequest;
use std::fmt::{self, Display, Formatter};

pub struct Request<'a>(pub &'a InvokeRequest);

impl Request<'_> {
    pub fn print(&self) -> String {
        todo!()
    }
}