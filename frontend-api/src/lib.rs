#![allow(clippy::trivially_copy_pass_by_ref)]

pub struct Client {
    endpoint: String,
    token: String,
}
use serde::{de::DeserializeOwned, Serialize};

impl Client {
    pub fn new(endpoint: String, token: String) -> Client {
        Client { endpoint, token }
    }

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
            .body(params)
            .send()?;
        res.json()
    }
}

include!(concat!(env!("OUT_DIR"), "/client_gen.rs"));
