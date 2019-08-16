//! Main purpose of `frontend-api` crate is now it's build script, which exposes frontend
//! GraphQL API schema. However, it also provides several useful abstractions
pub use frontend_api_derive::define_query;
use snafu::Snafu;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

struct InteractLogger {
    path: PathBuf,
    counter: AtomicU32,
}

impl InteractLogger {
    fn touch(&self) {
        std::fs::create_dir_all(&self.path).ok();
    }

    fn next(&self) -> String {
        self.counter.fetch_add(1, Ordering::SeqCst).to_string()
    }

    fn per_query(&self) -> QueryLogger {
        let path = self.path.join(self.next());
        std::fs::create_dir(&path).ok();
        QueryLogger { path }
    }
}

struct QueryLogger {
    path: PathBuf,
}

impl QueryLogger {
    fn log_request(&self, req: &[u8]) {
        let path = self.path.join("request.json");
        std::fs::write(path, req).ok();
    }

    fn log_response(&self, res: &[u8]) {
        let path = self.path.join("response.json");
        std::fs::write(path, res).ok();
    }
}

pub struct Client {
    host: String,
    port: u16,
    endpoint: String,
    token: String,
    interact_logger: Option<InteractLogger>,
}

pub struct Response<T>(graphql_client::Response<T>);

impl<T> Response<T> {
    pub fn is_fail(&self) -> bool {
        match &self.0.errors {
            Some(vec) => !vec.is_empty(),
            None => false,
        }
    }

    pub fn is_success(&self) -> bool {
        !self.is_fail()
    }

    pub fn into_inner(self) -> graphql_client::Response<T> {
        self.0
    }

    pub fn into_result(self) -> Result<T, Vec<graphql_client::Error>> {
        if self.is_success() {
            Ok(self.0.data.expect("No errors returned, but data is None"))
        } else {
            // it is guaranteed here that errors is Some
            // otherwise, is_success() must have been returned false
            Err(self.0.errors.unwrap())
        }
    }
}

#[derive(Debug, Snafu)]
pub enum TransportError {
    #[snafu(display("network error: {}", source))]
    Reqwest { source: reqwest::Error },
    #[snafu(display("ser/de error: {}", source))]
    Serde { source: serde_json::Error },
}

impl From<serde_json::Error> for TransportError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serde { source }
    }
}

impl From<reqwest::Error> for TransportError {
    fn from(source: reqwest::Error) -> Self {
        Self::Reqwest { source }
    }
}

impl Client {
    pub fn from_env() -> Client {
        use std::env::var;
        let host = var("JJS_API").unwrap_or_else(|_| "http://localhost".to_string());
        let port = var("JJS_API_PORT")
            .map_err(|_| ())
            .and_then(|s| s.parse().map_err(|_| ()))
            .unwrap_or(1779);
        let endpoint = var("JJS_API_ENDPOINT").unwrap_or_else(|_| "graphql".to_string());
        let token = var("JJS_AUTH").unwrap_or_else(|_| "Dev:User=Root".to_string());
        let mut cl = Client {
            host,
            port,
            endpoint,
            token,
            interact_logger: None,
        };

        if var("JJS_LOG_API").is_ok() {
            cl.enable_interact_logger();
        }

        cl
    }

    pub fn enable_interact_logger(&mut self) {
        let logger = InteractLogger {
            path: "./frontend-api-logs".into(),
            counter: AtomicU32::new(0),
        };
        logger.touch();
        self.interact_logger = Some(logger);
    }

    pub fn query<TQueryBody: serde::ser::Serialize, TResBody>(
        &self,
        req: &TQueryBody,
    ) -> Result<Response<TResBody>, TransportError>
    where
        TResBody: serde::de::DeserializeOwned,
    {
        let cl = reqwest::Client::new();
        let url = format!("{}:{}/{}", &self.host, &self.port, &self.endpoint);
        let query_logger = self.interact_logger.as_ref().map(InteractLogger::per_query);
        let req = serde_json::to_string(req)?;
        if let Some(ql) = &query_logger {
            ql.log_request(req.as_bytes())
        }
        let mut resp_data = cl
            .post(&url)
            .body(req)
            .header("Content-Type", "application/json")
            .header("X-Jjs-Auth", self.token.as_str())
            .send()?;
        let resp = resp_data.text()?;
        if let Some(ql) = &query_logger {
            ql.log_response(resp.as_bytes())
        }
        let resp = serde_json::from_str(&resp)?;
        Ok(Response(resp))
    }
}
