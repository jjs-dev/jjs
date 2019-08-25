use frontend_engine::{
    ApiServer,
    config,
};
use rocket::Rocket;
use std::env::temp_dir;
use std::path::PathBuf;

#[derive(Default)]
pub struct FixtureParams {
    toolchains: Vec<cfg::Toolchain>
}

impl FixtureParams {
    pub fn parse(s: &serde_yaml::Value) -> FixtureParams {
        let mut fp = FixtureParams::default();

        if let Some(langs) = s.get("toolchains") {
            let langs = langs.as_mapping().expect("env.toolchains must be map");
            for (k, v) in langs {
                let k = k.as_str().expect("toolchain name must be string");
                let v = v.as_str().expect("toolchain configuration must be given as string");
                let mut toolchain_cfg: cfg::Toolchain = toml::from_str(v).unwrap_or_else(|err| {
                    panic!("configuration for toolchain {} is invalid: {}", k, err);
                });
                toolchain_cfg.name = k.to_string();
                fp.toolchains.push(toolchain_cfg);
            }
        }

        fp
    }

    pub fn into_app(self, name: &str) -> Rocket {
        // TODO partially duplicates ApiServer::create_embedded()
        let db_conn = db::connect::connect_memory().unwrap();


        let path = temp_dir().join(format!("jjs-fr-eng-integ-test-{}", name));
        let path = path.to_str().expect("os temp dir is not utf8").to_string();

        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir(&path).expect("failed create dir for sysroot");

        init_jjs_root::init_jjs_root(init_jjs_root::Args {
            sysroot_dir: path.clone(),
            config_dir: None,
            symlink_config: false
        }).expect("failed initialize JJS sysroot");

        let contest = cfg::Contest {
            title: "DEV CONTEST".to_string(),
            problems: vec![
                cfg::Problem {
                    name: "dev-problem".to_string(),
                    code: "A".to_string(),
                    limits: Default::default(),
                    title: "DEV PROBLEM".to_string(),
                    loaded: true,
                }],
            group: "".to_string(),
            unregistered_visible: false,
            anon_visible: false,
        };

        let config = cfg::Config {
            toolchains: self.toolchains,
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
            env: config::Env::Dev, // TODO
        };

        ApiServer::create(frontend_config, logger, &config, db_conn.into())
    }
}