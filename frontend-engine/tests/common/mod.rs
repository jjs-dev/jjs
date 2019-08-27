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

impl Env {
    pub fn new(name: &str) -> Self {
        EnvBuilder::new().build(name)
    }

    pub fn exec(&self, operation: &str, vars: &serde_json::Value) -> serde_json::Value {
        let obj = serde_json::json!({
             "query": operation,
             "variables": vars,
        });
        let body = serde_json::to_string(&obj).unwrap();
        let request = self
            .client
            .post("/graphql")
            .body(body)
            .header(rocket::http::ContentType::JSON);

        let mut response = request.dispatch();
        assert_eq!(response.status(), rocket::http::Status::Ok);
        assert_eq!(
            response.content_type(),
            Some("application/json".parse().unwrap())
        );
        let body = response.body_string().unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        body
    }

    fn check_ok(res: serde_json::Value) -> serde_json::Value {
        if util::is_ok(&res) {
            res.get("data").unwrap().clone()
        } else {
            let errs = res
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

    pub fn exec_ok(&self, req: &str) -> serde_json::Value {
        let res = self.exec(req, &serde_json::Value::Object(Default::default()));
        Self::check_ok(res)
    }

    pub fn exec_ok_with_vars(&self, req: &str, vars: &serde_json::Value) -> serde_json::Value {
        assert!(vars.is_object());
        let res = self.exec(req, vars);
        Self::check_ok(res)
    }

    pub fn exec_err(&self, req: &str) -> Vec<serde_json::Value> {
        let res = self.exec(req, &serde_json::Value::Object(Default::default()));
        if util::is_ok(&res) {
            eprintln!("Error: query with fail=true succeeded");
            eprintln!("Response: \n{:?}", res);
            panic!("Operation succeeded unexpectedly");
        } else {
            let errs = res.get("errors").unwrap().as_array().unwrap();
            assert!(!errs.is_empty());
            errs.clone()
        }
    }
}

pub mod util {
    pub fn is_ok(res: &serde_json::Value) -> bool {
        res.get("errors").is_none()
    }

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
