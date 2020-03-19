// This file is included in many tests, and some functions are not used in all tests
#![allow(dead_code)]
use frontend_engine::{config, test_util, ApiServer};
pub use test_util::check_error;

use std::{env::temp_dir, path::PathBuf, sync::Arc};

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

    pub fn build(&mut self, name: &str) -> Env {
        util::log::setup();
        // TODO partially duplicates ApiServer::create_embedded()
        let db_conn: Arc<dyn db::DbConn> = db::connect::connect_memory().unwrap().into();

        let path = temp_dir().join(format!("jjs-fr-eng-integ-test-{}", name));

        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir(&path).expect("failed create dir for sysroot");

        let runner = util::cmd::Runner::new();

        setup::setup(
            &setup::SetupParams {
                toolchains: false,
                data_dir: Some(path.clone()),
                config: None,
                db: None,
                // dummy value can be used because we don't setup db
                install_dir: PathBuf::new(),
                force: false,
                sample_contest: false,
            },
            &runner,
        )
        .expect("failed initialize JJS sysroot");

        runner.exit_if_errors();

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

        let secret = config::derive_key_512("EMBEDDED_FRONTEND_INSTANCE");
        let frontend_config = config::FrontendParams {
            cfg: config::FrontendConfig {
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
            token_mgr: frontend_engine::TokenMgr::new(db_conn.clone(), secret.into()),
        };

        let rock = ApiServer::create(
            Arc::new(frontend_config),
            self.inner
                .take()
                .expect("EnvBuilder can not be used more than once")
                .into_inner(),
            db_conn,
            problem_loader::Loader::empty(),
            &path,
        );
        Env {
            client: rocket::local::Client::new(rock.unwrap()).unwrap(),
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

    pub fn operation(&mut self, op: &str) -> &mut Self {
        self.builder.operation(op);
        self
    }

    pub fn auth(&mut self, token: impl ToString) -> &mut Self {
        self.auth_token = Some(token.to_string());
        self
    }

    pub fn exec(&self) -> test_util::Response {
        let body = self.builder.to_query();
        let request = self
            .client
            .post("/graphql")
            .body(body)
            .header(rocket::http::Header::new(
                "X-Jjs-Auth",
                self.auth_token
                    .clone()
                    .unwrap_or_else(|| "Dev root".to_string()),
            ))
            .header(rocket::http::ContentType::JSON);

        let mut response = request.dispatch();
        if response.status() != rocket::http::Status::Ok {
            eprintln!("Frontend returned non-200: {:?}", response.status());
            eprintln!("Response: {}", response.body_string().unwrap_or_default());
            panic!()
        }
        assert_eq!(
            response.content_type(),
            Some("application/json".parse().unwrap())
        );
        let body = response.body_string().unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        test_util::Response(body)
    }
}

impl Env {
    pub fn new(name: &str) -> Self {
        EnvBuilder::new().build(name)
    }

    pub fn req(&self) -> RequestBuilder {
        RequestBuilder {
            builder: test_util::RequestBuilder::new(),
            auth_token: None,
            client: &self.client,
        }
    }
}
