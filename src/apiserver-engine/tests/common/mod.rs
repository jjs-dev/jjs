// This file is included in many tests, and some functions are not used in all
// tests
#![allow(dead_code)]
use apiserver_engine::{config, test_util, ApiServer};
use setup::Component;
pub use test_util::check_error;

pub struct EnvBuilder {
    inner: Option<entity::loader::LoaderBuilder>,
}

impl Default for EnvBuilder {
    fn default() -> Self {
        EnvBuilder {
            inner: Some(entity::loader::LoaderBuilder::new()),
        }
    }
}

fn is_caused_by_used_port(err: &apiserver_engine::ApiServerCreateError) -> bool {
    let err = match err {
        apiserver_engine::ApiServerCreateError::Io(io) => io,
        _ => return false,
    };
    matches!(err.kind(), std::io::ErrorKind::AddrInUse)
}

impl EnvBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    fn get(&mut self) -> &mut entity::loader::LoaderBuilder {
        self.inner
            .as_mut()
            .expect("EnvBuilder can not be used more than once")
    }

    pub fn toolchain(&mut self, tc: entity::Toolchain) -> &mut Self {
        self.get().put(tc);
        self
    }

    pub fn contest(&mut self, contest: entity::Contest) -> &mut Self {
        self.get().put(contest);
        self
    }

    pub async fn build(&mut self) -> Env {
        simple_logger::init().ok();

        let tempdir = tempfile::tempdir().expect("failed to create temporary dir");

        let db_conn: db::DbConn = db::connect::connect_memory().unwrap();

        std::fs::remove_dir_all(tempdir.path()).ok();
        std::fs::create_dir(tempdir.path()).expect("failed create dir for sysroot");
        {
            let cx = setup::data::Context {
                data_dir: tempdir.path(),
            };
            let upgrader = setup::data::analyze(cx)
                .await
                .expect("failed to create upgrader");
            upgrader
                .upgrade()
                .await
                .expect("failed to initialize JJS data_dir");
        }

        let contest = entity::Contest {
            id: "main".to_string(),
            title: "DEV CONTEST".to_string(),
            problems: vec![entity::entities::contest::ProblemBinding {
                name: "dev-problem".to_string(),
                code: "A".to_string(),
            }],
            participants: vec!["Participants".to_string()],
            unregistered_visible: false,
            anon_visible: false,
            judges: vec!["Judges".to_string()],
            is_virtual: false,
            start_time: None,
            end_time: None,
            duration: None,
        };

        self.get().put(contest);

        let secret = config::derive_key_512("EMBEDDED_APISERVER_INSTANCE");
        let mut apiserver_config = config::ApiserverConfig {
            listen: config::ListenConfig {
                port: 0,
                host: "127.0.0.1".to_string(),
            },
            unix_socket_path: "".to_string(),
            env: config::Env::Dev,
            external_addr: Some("127.0.0.1".to_string()),
            tls: None,
        };

        let token_manager = apiserver_engine::TokenMgr::new(db_conn.clone(), secret.into());
        for _ in 0u8..10u8 {
            let mut port: u16 = 0;
            while port <= 1024 {
                port = rand::random();
            }
            apiserver_config.listen.port = port;
            let params = apiserver_engine::ApiserverParams {
                token_manager: token_manager.clone(),
                config: apiserver_config.clone(),
                entity_loader: self
                    .inner
                    .take()
                    .expect("EnvBuilder can not be used more than once")
                    .into_inner(),
                problem_loader: problem_loader::Loader::empty(),
                data_dir: tempdir.path().to_path_buf(),
                db_conn: db_conn.clone(),
                tls_mode: apiserver_engine::TlsMode::Disabled,
            };
            let maybe_server = ApiServer::create(params).await;

            match maybe_server {
                Ok(server) => {
                    return Env {
                        endpoint: format!("http://127.0.0.1:{}", port),
                        server,
                        tempdir,
                    };
                }
                Err(err) => {
                    if is_caused_by_used_port(&err) {
                        continue;
                    } else {
                        panic!("Failed to bind to port {}: {}", port, err)
                    }
                }
            }
        }
        panic!("Failed to find free port")
    }
}

pub struct Env {
    endpoint: String,
    server: ApiServer,
    tempdir: tempfile::TempDir,
}
#[derive(Debug)]
pub struct RequestBuilder {
    builder: test_util::RequestBuilder,
    auth_token: Option<String>,
    endpoint: String,
}

impl RequestBuilder {
    pub fn var(&mut self, name: &str, val: impl Into<serde_json::Value>) -> &mut Self {
        self.builder.var(name, &val.into());
        self
    }

    pub fn action(&mut self, op: &str) -> &mut Self {
        self.builder.action(op);
        self
    }

    pub fn auth(&mut self, token: impl ToString) -> &mut Self {
        self.auth_token = Some(format!("Token {}", token.to_string()));
        self
    }

    pub fn method(&mut self, m: apiserver_engine::test_util::Method) -> &mut Self {
        self.builder.method(m);
        self
    }

    pub async fn exec(&self) -> test_util::Response {
        let url = self.builder.action.clone().expect("no action was provided");
        let url = format!("{}{}", self.endpoint, url);
        let client = reqwest::Client::new();
        let mut request = if self.builder.body.is_empty() {
            match self.builder.method {
                Some(apiserver_engine::test_util::Method::Delete) => client.delete(&url),
                None => client.get(&url),
                _ => unreachable!(),
            }
        } else {
            match self.builder.method {
                Some(apiserver_engine::test_util::Method::Patch) => client.patch(&url),
                None => client.post(&url),
                _ => unreachable!(),
            }
        };
        if !self.builder.body.is_empty() {
            request = request
                .body(serde_json::to_string(&self.builder.body).expect("serialize request body"));
        }
        let request = request
            .header(
                "Authorization",
                self.auth_token
                    .clone()
                    .unwrap_or_else(|| "Token Dev::root".to_string()),
            )
            .header("Content-Type", "application/json");

        let response = request.send().await.expect("failed to send request");
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return test_util::Response(serde_json::Value::Null);
        }
        let content_type = response
            .headers()
            .get("Content-Type")
            .map(reqwest::header::HeaderValue::as_bytes);
        if content_type != Some(b"application/json") {
            eprintln!("Apiserver returned non-json: {:?}", content_type);
            eprintln!("Response: {}", response.text().await.unwrap_or_default());
            panic!()
        }
        let body = response.text().await.unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        test_util::Response(body)
    }
}

impl Env {
    pub async fn new() -> Self {
        EnvBuilder::new().build().await
    }

    pub fn req(&self) -> RequestBuilder {
        RequestBuilder {
            builder: test_util::RequestBuilder::new(),
            auth_token: None,
            endpoint: self.endpoint.clone(),
        }
    }
}
