// This file is included in many tests, and some functions are not used in all tests
#![allow(dead_code)]
use frontend_engine::{config, test_util, ApiServer};
pub use test_util::check_error;

use std::{env::temp_dir, path::PathBuf, sync::Arc};

#[derive(Default)]
pub struct EnvBuilder {
    toolchains: Vec<cfg::Toolchain>,
}

impl EnvBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn toolchain(&mut self, tc: cfg::Toolchain) -> &mut Self {
        self.toolchains.push(tc);
        self
    }

    pub fn build(&self, name: &str) -> Env {
        util::log::setup();
        // TODO partially duplicates ApiServer::create_embedded()
        let db_conn: Arc<dyn db::DbConn> = db::connect::connect_memory().unwrap().into();

        let path = temp_dir().join(format!("jjs-fr-eng-integ-test-{}", name));
        let path = path.to_str().expect("os temp dir is not utf8").to_string();

        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir(&path).expect("failed create dir for sysroot");

        let runner = util::cmd::Runner::new();

        setup::setup(
            &setup::SetupParams {
                toolchains: false,
                data_dir: Some(path.clone().into()),
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

        let contest = cfg::Contest {
            title: "DEV CONTEST".to_string(),
            problems: vec![cfg::Problem {
                name: "dev-problem".to_string(),
                code: "A".to_string(),
                limits: Default::default(),
                title: "DEV PROBLEM".to_string(),
                loaded: true,
            }],
            group: Vec::new(),
            unregistered_visible: false,
            anon_visible: false,
            judges: Vec::new(),
        };

        let config = cfg::Config {
            toolchains: self.toolchains.clone(),
            sysroot: PathBuf::from(path),
            install_dir: Default::default(),
            toolchain_root: "".to_string(),
            global_env: Default::default(),
            env_passing: false,
            env_blacklist: vec![],
            contests: vec![contest],
            problems: Default::default(),
        };
        let secret = config::derive_key_512("EMBEDDED_FRONTEND_INSTANCE");
        let frontend_config = config::FrontendConfig {
            port: 0,
            host: "127.0.0.1".to_string(),
            unix_socket_path: "".to_string(),
            env: config::Env::Dev,
            db_conn: db_conn.clone(),
            token_mgr: frontend_engine::TokenMgr::new(db_conn.clone(), secret.into()),
            addr: Some("127.0.0.1".to_string()),
        };

        let rock = ApiServer::create(frontend_config, &config, db_conn);
        Env {
            client: rocket::local::Client::new(rock).unwrap(),
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
