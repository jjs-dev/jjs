#[macro_use]
extern crate serde_derive;

pub mod base;
pub mod util;
pub mod auth;
pub mod submission;

pub use self::base::*;
pub use self::submission::{SubmissionRequest, SubmissionResult};

#[derive(Debug, Serialize, Deserialize)]
pub enum RequestBody {
    Ping(util::PingRequest),
    PasswordAuth(auth::PasswordAuthRequest),
    Submission(SubmissionRequest),
}


#[derive(Debug, Serialize, Deserialize)]
pub enum ResponseBody {
    Ping(util::PingResult),
    PasswordAuth(auth::PasswordAuthResult),
    Submission(SubmissionResult),
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub query: RequestBody,
    pub auth: Auth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub result: ResponseBody,
}