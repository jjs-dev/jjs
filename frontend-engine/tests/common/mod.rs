use frontend_engine::{config, ApiServer};

use std::{env::temp_dir, path::PathBuf};

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
        // TODO partially duplicates ApiServer::create_embedded()
        let db_conn = db::connect::connect_memory().unwrap();

        let path = temp_dir().join(format!("jjs-fr-eng-integ-test-{}", name));
        let path = path.to_str().expect("os temp dir is not utf8").to_string();

        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir(&path).expect("failed create dir for sysroot");

        init_jjs_root::init_jjs_root(init_jjs_root::Args {
            sysroot_dir: path.clone(),
            config_dir: None,
            symlink_config: false,
        })
        .expect("failed initialize JJS sysroot");

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
        let logger = slog::Logger::root(slog::Discard, slog::o!());
        let frontend_config = config::FrontendConfig {
            port: 0,
            host: "127.0.0.1".to_string(),
            secret: config::derive_key_512("EMBEDDED_FRONTEND_INSTANCE"),
            unix_socket_path: "".to_string(),
            env: config::Env::Dev,
        };

        let rock = ApiServer::create(frontend_config, logger, &config, db_conn.into());
        Env {
            client: rocket::local::Client::new(rock).unwrap(),
        }
    }
}

pub struct Env {
    client: rocket::local::Client,
}

pub struct RequestBuilder<'a> {
    vars: Option<serde_json::Value>,
    auth_token: Option<String>,
    operation: Option<String>,
    client: &'a rocket::local::Client,
}

impl RequestBuilder<'_> {
    pub fn vars(&mut self, v: &serde_json::Value) -> &mut Self {
        assert!(v.is_object());
        self.vars = Some(v.clone());
        self
    }

    pub fn operation(&mut self, op: &str) -> &mut Self {
        self.operation = Some(op.to_string());
        self
    }

    pub fn exec(&self) -> Response {
        let obj = serde_json::json!({
             "query": self.operation.as_ref().unwrap(),
             "variables": self.vars.clone().unwrap_or_else(||serde_json::Value::Null),
        });
        let body = serde_json::to_string(&obj).unwrap();
        let request = self
            .client
            .post("/graphql")
            .body(body)
            .header(rocket::http::Header::new(
                "X-Jjs-Auth",
                self.auth_token
                    .clone()
                    .unwrap_or_else(|| "Dev root".to_string())
                    .to_string(),
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
        Response(body)
    }
}

#[derive(Debug, Clone)]
pub struct Response(serde_json::Value);

impl std::ops::Deref for Response {
    type Target = serde_json::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Response {
    pub fn is_ok(&self) -> bool {
        self.0.get("errors").is_none()
    }

    pub fn unwrap_ok(self) -> serde_json::Value {
        if self.is_ok() {
            self.0.get("data").unwrap().clone()
        } else {
            let errs = self
                .0
                .get("errors")
                .expect("errors missing on failed request")
                .as_array()
                .expect("errors field must be array");
            assert!(!errs.is_empty());
            eprintln!("Error: query failed");
            eprintln!("Server response contains errors:");
            for (i, err) in errs.iter().enumerate() {
                if i != 0 {
                    eprintln!("------");
                }
                util::print_error(&err);
            }
            panic!("Operation failed unexpectedly");
        }
    }

    pub fn unwrap_errs(self) -> Vec<serde_json::Value> {
        if self.is_ok() {
            eprintln!("Error: query with fail=true succeeded");
            eprintln!("Response: \n{:?}", self.0);
            panic!("Operation succeeded unexpectedly");
        } else {
            let errs = self.0.get("errors").unwrap().as_array().unwrap();
            assert!(!errs.is_empty());
            errs.clone()
        }
    }
}

impl Env {
    pub fn new(name: &str) -> Self {
        EnvBuilder::new().build(name)
    }

    pub fn req(&self) -> RequestBuilder {
        RequestBuilder {
            vars: None,
            auth_token: None,
            operation: None,
            client: &self.client,
        }
    }
}

pub mod util {
    pub fn print_error(err: &serde_json::Value) {
        let mut err = err.as_object().unwrap().clone();
        let ext = err.remove("extensions");
        println!("{}", serde_json::to_string_pretty(&err).unwrap());
        if let Some(ext) = ext {
            if let Some(ext) = ext.as_object() {
                println!("extensions:\n");
                if let Some(error_code) = ext.get("errorCode") {
                    println!("error code: {}", error_code.to_string());
                }
                if let Some(backtrace) = ext.get("trace") {
                    println!("backtrace: {}", backtrace.as_str().unwrap());
                }
            }
        }
    }

    pub fn check_error(err: &serde_json::Value, exp_code: &str) {
        let code = err
            .get("extensions")
            .and_then(|v| v.get("errorCode"))
            .and_then(|v| v.as_str())
            .map(|x| x.to_string());
        assert_eq!(code.as_ref().map(String::as_str), Some(exp_code));
    }
}
