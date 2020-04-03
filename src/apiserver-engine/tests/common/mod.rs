// This file is included in many tests, and some functions are not used in all tests
#![allow(dead_code)]
use apiserver_engine::{config, test_util, ApiServer};
use setup::Component;
pub use test_util::check_error;

use std::{env::temp_dir, sync::Arc};

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

    pub async fn build(&mut self, name: &str) -> Env {
        simple_logger::init().ok();
        // TODO partially duplicates ApiServer::create_embedded()
        let db_conn: Arc<db::DbConn> = db::connect::connect_memory().unwrap().into();

        let path = temp_dir().join(format!("jjs-fr-eng-integ-test-{}", name));

        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir(&path).expect("failed create dir for sysroot");
        {
            let cx = setup::data::Context { data_dir: &path };
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
            group: Vec::new(),
            unregistered_visible: false,
            anon_visible: false,
            judges: Vec::new(),
        };

        self.get().put(contest);

        let secret = config::derive_key_512("EMBEDDED_APISERVER_INSTANCE");
        let apiserver_config = config::ApiserverParams {
            cfg: config::ApiserverConfig {
                listen: config::ListenConfig {
                    port: 0,
                    host: "127.0.0.1".to_string(),
                },
                unix_socket_path: "".to_string(),
                env: config::Env::Dev,
                external_addr: Some("127.0.0.1".to_string()),
                tls: None,
            },

            db_conn: db_conn.clone(),
            token_mgr: apiserver_engine::TokenMgr::new(db_conn.clone(), secret.into()),
        };

        let mut server = ApiServer::create(
            Arc::new(apiserver_config),
            self.inner
                .take()
                .expect("EnvBuilder can not be used more than once")
                .into_inner(),
            db_conn,
            problem_loader::Loader::empty(),
            &path,
        )
        .unwrap();
        Env {
            client: rocket::local::Client::new(server.take_rocket()).unwrap(),
        }
    }
}

pub struct Env {
    client: rocket::local::Client,
}

pub struct RequestBuilder<'a> {
    builder: test_util::RequestBuilder,
    auth_token: Option<String>,
    client: &'a rocket::local::Client,
}

impl std::fmt::Debug for RequestBuilder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("ReguestBuidler")
            .field("builder", &self.builder)
            .field("auth_token", &self.auth_token)
            .finish()
    }
}

impl RequestBuilder<'_> {
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

    pub async fn exec(&self) -> test_util::Response {
        let url = self.builder.action.clone().expect("no action was provided");
        let request = if self.builder.body.is_empty() {
            self.client.get(url)
        } else {
            self.client
                .post(url)
                .body(serde_json::to_string(&self.builder.body).expect("serialize request body"))
        };
        let request = request
            .header(rocket::http::Header::new(
                "Authorization",
                self.auth_token
                    .clone()
                    .unwrap_or_else(|| "Token Dev::root".to_string()),
            ))
            .header(rocket::http::ContentType::JSON);

        let mut response = request.dispatch().await;
        if response.content_type() != Some("application/json".parse().unwrap()) {
            eprintln!("Apiserver returned non-json: {:?}", response.content_type());
            eprintln!(
                "Response: {}",
                response.body_string().await.unwrap_or_default()
            );
            panic!()
        }
        let body = response.body_string().await.unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        test_util::Response(body)
    }
}

impl Env {
    pub async fn new(name: &str) -> Self {
        EnvBuilder::new().build(name).await
    }

    pub fn req(&self) -> RequestBuilder {
        RequestBuilder {
            builder: test_util::RequestBuilder::new(),
            auth_token: None,
            client: &self.client,
        }
    }
}
