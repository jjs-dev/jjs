#![feature(proc_macro_hygiene, decl_macro)]

use rocket::{catch, catchers, fairing::AdHoc, get, routes, Rocket};
use std::sync::Arc;
use thiserror::Error;

mod api;
pub mod config;
pub mod introspect;
mod password;
pub mod root_auth;
pub mod secret_key;
pub mod test_util;

pub use api::TokenMgr;
pub use config::ApiserverParams;

type DbPool = Arc<db::DbConn>;

#[catch(400)]
fn catch_bad_request() -> &'static str {
    r#"
Your request is incorrect.
Possible reasons:
- Query body is missing or is not valid JSON
- `Authorization` header does not contain access token
    "#
}

#[get("/")]
fn route_ping() -> &'static str {
    "JJS apiserver: pong"
}

#[derive(Error, Debug)]
pub enum ApiServerCreateError {
    #[error("failed to initialize Rocket: {0}")]
    Rocket(#[from] rocket::config::ConfigError),
}

pub struct ApiServer {
    rocket: Option<Rocket>,
}

impl ApiServer {
    pub fn create_embedded() -> ApiServer {
        let db_conn: Arc<db::DbConn> = db::connect::connect_memory().unwrap().into();
        let builder = entity::loader::LoaderBuilder::new();
        let secret: Arc<[u8]> = config::derive_key_512("EMBEDDED_APISERVER_INSTANCE")
            .into_boxed_slice()
            .into();
        let token_mgr = crate::api::TokenMgr::new(db_conn.clone(), secret);
        let apiserver_config = config::ApiserverParams {
            cfg: config::ApiserverConfig {
                listen: config::ListenConfig {
                    host: "127.0.0.1".to_string(),
                    port: 0,
                },
                external_addr: Some("127.0.0.1".to_string()),
                unix_socket_path: "".to_string(),
                env: config::Env::Dev,
                tls: None,
            },
            token_mgr,
            db_conn: db_conn.clone(),
        };

        Self::create(
            Arc::new(apiserver_config),
            builder.into_inner(),
            db_conn,
            problem_loader::Loader::empty(),
            std::path::Path::new("/tmp/jjs"),
        )
        .expect("failed to create embedded instance")
    }

    pub fn create(
        apiserver_params: Arc<config::ApiserverParams>,
        entity_loader: entity::Loader,
        pool: DbPool,
        problem_loader: problem_loader::Loader,
        data_dir: &std::path::Path,
    ) -> Result<ApiServer, ApiServerCreateError> {
        let rocket_cfg_env = match apiserver_params.cfg.env {
            config::Env::Prod => rocket::config::Environment::Production,
            config::Env::Dev => rocket::config::Environment::Development,
        };
        let mut rocket_config = rocket::Config::new(rocket_cfg_env);

        rocket_config.set_address(apiserver_params.cfg.listen.host.clone())?;
        rocket_config.set_port(apiserver_params.cfg.listen.port);
        rocket_config.set_log_level(match apiserver_params.cfg.env {
            config::Env::Dev => rocket::config::LoggingLevel::Normal,
            config::Env::Prod => rocket::config::LoggingLevel::Critical,
        });
        rocket_config
            .set_secret_key(base64::encode(apiserver_params.token_mgr.secret_key()))
            .unwrap();
        if let Some(tls) = &apiserver_params.cfg.tls {
            rocket_config.set_tls(&tls.cert_path, &tls.key_path)?;
        }

        let graphql_context_factory = api::ContextFactory {
            pool: Arc::clone(&pool),
            cfg: Arc::new(entity_loader),
            problem_loader: Arc::new(problem_loader),
            data_dir: data_dir.into(),
        };

        let cfg1 = Arc::clone(&apiserver_params);
        let rocket = rocket::custom(rocket_config)
            .manage(graphql_context_factory)
            .manage(apiserver_params)
            .attach(AdHoc::on_attach("ProvideSecretKey", move |rocket| {
                Ok(rocket.manage(secret_key::SecretKey(cfg1.token_mgr.secret_key().into())))
            }))
            .mount(
                "/",
                routes![
                    route_ping,
                    api::misc::route_get_api_version,
                    api::misc::route_is_dev,
                    api::contests::route_get,
                    api::contests::route_list,
                    api::contests::route_list_problems,
                    api::contests::route_get_participation,
                    api::contests::route_update_participation,
                    api::runs::route_list,
                    api::runs::route_get,
                    api::runs::route_submit_simple,
                    api::runs::route_patch,
                    api::runs::route_live,
                    api::runs::route_delete,
                    api::runs::route_protocol,
                    api::runs::route_source,
                    api::runs::route_binary,
                    api::monitor::route_get,
                    api::toolchains::route_list,
                    api::auth::route_simple,
                    api::users::route_create
                ],
            )
            .register(catchers![catch_bad_request]);
        Ok(ApiServer {
            rocket: Some(rocket),
        })
    }

    pub fn take_rocket(&mut self) -> Rocket {
        std::mem::take(&mut self.rocket).expect("ApiServer: rocket is already taken")
    }
}
