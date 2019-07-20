#![allow(clippy::trivially_copy_pass_by_ref, clippy::ptr_arg)]

use slog::Logger;

pub struct Client {
    pub endpoint: String,
    pub token: String,
    pub logger: Option<Logger>,
}

use serde::{de::DeserializeOwned, Serialize};

impl Client {
    fn exec_query<In: Serialize, Out: DeserializeOwned>(
        &self,
        method: &str,
        params: &In,
    ) -> Result<Out, reqwest::Error> {
        let url = format!("{}/{}", self.endpoint, method);
        let params = serde_json::to_string(params).unwrap();
        let rw = reqwest::Client::new();
        let mut res = rw
            .post(&url)
            .header("X-JJS-Auth", self.token.as_str())
            .body(params.clone())
            .send()?;
        if let Some(ref logger) = self.logger {
            slog::debug!(logger, "JJS frontend query"; "method" => method, "params" => ?params, "result" => ?res);
        }
        res.json()
    }
}

pub trait FrontendError: std::error::Error + std::fmt::Debug {}

pub type NetworkError = reqwest::Error;

include!(concat!(env!("OUT_DIR"), "/client_gen.rs"));
