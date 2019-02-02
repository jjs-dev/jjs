use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
};

pub trait QueryJjs {
    type TransportError;
    fn call(
        &mut self,
        q: &frontend_api::Request,
    ) -> Result<frontend_api::Response, Self::TransportError>;
}

impl From<reqwest::Error> for TransportError {
    fn from(e: reqwest::Error) -> TransportError {
        TransportError::Io(e)
    }
}

impl From<serde_json::Error> for TransportError {
    fn from(e: serde_json::Error) -> TransportError {
        TransportError::Serde(e)
    }
}

pub struct JjsApiClient {
    cl: reqwest::Client,
    addr: String,
}

impl JjsApiClient {
    pub fn from_endpoint(addr: String) -> JjsApiClient {
        JjsApiClient {
            addr,
            cl: reqwest::Client::new(),
        }
    }
}

impl QueryJjs for JjsApiClient {
    type TransportError = TransportError;

    fn call(
        &mut self,
        q: &frontend_api::Request,
    ) -> Result<frontend_api::Response, TransportError> {
        //let mut s = TcpStream::connect(self.addr)?;
        let mut req = self.cl.post(&self.addr);
        let q = serde_json::to_string(q)?;
        let q = q.into_bytes();
        req = req.body(q);
        let mut res = req.send()?;
        let r: frontend_api::Response = res.json()?;
        Ok(r)
    }
}

//TODO write
/*
macro_rules! jjs_unwrap {
    ($var:ident, $tag:ident) => {
        match var {
            frontend_api::
        }
    };
}*/
