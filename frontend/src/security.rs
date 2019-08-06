mod access_control;
mod acl_query;
mod token;

pub use access_control::{init, AccessControlData};
pub use token::{AccessCheckService, Token, TokenFromRequestError};

#[derive(Clone)]
pub struct SecretKey(pub Vec<u8>);

pub const AUTH_HEADER_NAME: &str = "X-JJS-Auth";
