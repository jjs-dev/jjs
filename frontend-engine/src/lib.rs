#![feature(proc_macro_hygiene, decl_macro)]

use rocket::{catch, catchers, get, post, routes, Rocket};

pub mod config;
mod gql_server;
mod password;
pub mod root_auth;
mod security;

pub use config::FrontendConfig;
pub use root_auth::LocalAuthServer;

use gql_server::Context;
use rocket::{fairing::AdHoc, State};
use slog::Logger;
use std::sync::Arc;

type DbPool = Arc<dyn db::DbConn>;

#[catch(400)]
fn catch_bad_request() -> &'static str {
    r#"
Your request is incorrect.
Possible reasons:
- Query body is missing or is not valid JSON
- X-Jjs-Auth header is not valid access token
    "#
}

#[get("/")]
fn route_ping() -> &'static str {
    "JJS frontend: pong"
}

#[get("/graphiql")]
fn route_graphiql() -> rocket::response::content::Html<String> {
    juniper_rocket::graphiql_source("/graphql")
}

struct JuniperResponseDebug(juniper_rocket::GraphQLResponse);

impl std::fmt::Debug for JuniperResponseDebug {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let resp = &self.0;
        std::fmt::Debug::fmt(&resp.1, f)
    }
}

fn execute_request(
    req: juniper_rocket::GraphQLRequest,
    schema: &gql_server::Schema,
    ctx: &gql_server::Context,
) -> juniper_rocket::GraphQLResponse {
    let res = req.execute(schema, ctx);

    let res = JuniperResponseDebug(res);

    slog::debug!(ctx.logger, "API request"; "request" => ?req, "response" => ?res);

    res.0
}

#[get("/graphql?<request>")]
fn route_get_graphql(
    ctx: gql_server::Context,
    request: juniper_rocket::GraphQLRequest,
    schema: State<gql_server::Schema>,
) -> juniper_rocket::GraphQLResponse {
    execute_request(request, &*schema, &ctx)
}

#[post("/graphql", data = "<request>")]
fn route_post_graphql(
    ctx: gql_server::Context,
    request: juniper_rocket::GraphQLRequest,
    schema: State<gql_server::Schema>,
) -> juniper_rocket::GraphQLResponse {
    execute_request(request, &*schema, &ctx)
}

#[derive(Clone)]
struct GqlApiSchema(String);

#[rocket::get("/graphql/schema")]
fn route_get_graphql_schema(schema: State<GqlApiSchema>) -> String {
    schema.clone().0
}

pub struct ApiServer {}

impl ApiServer {
    pub fn create_embedded() -> Rocket {
        let db_conn = db::connect::connect_memory().unwrap();

        let config = cfg::Config {
            toolchains: vec![],
            sysroot: Default::default(),
            install_dir: Default::default(),
            toolchain_root: "".to_string(),
            global_env: Default::default(),
            env_passing: false,
            env_blacklist: vec![],
            contests: vec![],
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

        Self::create(frontend_config, logger, &config, db_conn.into())
    }

    pub fn get_schema() -> String {
        let rock = Self::create_embedded();
        rock.state::<GqlApiSchema>().unwrap().0.clone()
    }

    pub fn create(
        frontend_config: config::FrontendConfig,
        logger: Logger,
        config: &cfg::Config,
        pool: DbPool,
    ) -> Rocket {
        let rocket_cfg_env = match frontend_config.env {
            config::Env::Prod => rocket::config::Environment::Production,
            config::Env::Dev => rocket::config::Environment::Development,
        };
        let mut rocket_config = rocket::Config::new(rocket_cfg_env);

        rocket_config
            .set_address(frontend_config.host.clone())
            .unwrap();
        rocket_config.set_port(frontend_config.port);
        rocket_config.set_log_level(match frontend_config.env {
            config::Env::Dev => rocket::config::LoggingLevel::Normal,
            config::Env::Prod => rocket::config::LoggingLevel::Critical,
        });
        rocket_config
            .set_secret_key(base64::encode(&frontend_config.secret))
            .unwrap();

        let graphql_context_factory = gql_server::ContextFactory {
            pool: Arc::clone(&pool),
            cfg: std::sync::Arc::new(config.clone()),
            logger,
        };

        let graphql_schema = gql_server::Schema::new(gql_server::Query, gql_server::Mutation);

        let (intro_data, intro_errs) = juniper::introspect(
            &graphql_schema,
            &Context(Arc::new(
                graphql_context_factory.create_context_data_unrestricted(),
            )),
            juniper::IntrospectionFormat::default(),
        )
        .unwrap();
        assert!(intro_errs.is_empty());

        let introspection_json = serde_json::to_string(&intro_data).unwrap();

        let cfg1 = frontend_config.clone();
        let cfg2 = frontend_config.clone();

        rocket::custom(rocket_config)
            .manage(graphql_context_factory)
            .manage(graphql_schema)
            .manage(GqlApiSchema(introspection_json))
            .attach(AdHoc::on_attach("ProvideSecretKey", move |rocket| {
                Ok(rocket.manage(security::SecretKey(cfg1.secret.clone().into())))
            }))
            .attach(AdHoc::on_attach("RegisterEnvironmentKind", move |rocket| {
                Ok(rocket.manage(cfg2.env))
            }))
            .mount(
                "/",
                routes![
                    route_get_graphql_schema,
                    route_graphiql,
                    route_get_graphql,
                    route_post_graphql,
                    route_ping,
                ],
            )
            .register(catchers![catch_bad_request])
    }
}
