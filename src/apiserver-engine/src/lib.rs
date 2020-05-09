//TODO: remove
#![feature(proc_macro_hygiene, decl_macro, type_alias_impl_trait)]

mod api;
pub mod config;
pub mod introspect;
mod password;
pub mod root_auth;
pub mod secret_key;
pub mod test_util;

pub use api::TokenMgr;

use log::debug;
use thiserror::Error;

async fn route_ping() -> &'static str {
    "JJS apiserver: pong"
}

#[derive(Error, Debug)]
pub enum ApiServerCreateError {
    #[error("bind address '{1}' is invalid: {0}")]
    Ip(std::net::AddrParseError, String),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("ssl initialization failed")]
    Ssl(#[from] daemons::ssl::CreateSslBuilderError),
}

#[derive(Clone)]
pub struct ShutdownHandle {
    chan: tokio::sync::mpsc::UnboundedSender<()>,
}

impl ShutdownHandle {
    pub fn shutdown(self) {
        // send error means that server is already shut down.
        // this is OK.
        self.chan.send(()).ok();
    }
}

pub struct ApiServer {
    shutdown: ShutdownHandle,
}

pub enum TlsMode {
    Forced,
    Enabled,
    Disabled,
}

pub struct ApiserverParams {
    pub token_manager: TokenMgr,
    pub config: config::ApiserverConfig,
    pub entity_loader: entity::Loader,
    pub db_conn: db::DbConn,
    pub problem_loader: problem_loader::Loader,
    pub data_dir: std::path::PathBuf,
    pub tls_mode: TlsMode,
    pub single_worker: bool,
}

impl ApiServer {
    #[actix_rt::main]
    async fn serve(
        params: ApiserverParams,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<()>,
        startup_tx: tokio::sync::oneshot::Sender<()>,
    ) -> Result<(), ApiServerCreateError> {
        let listen_address: std::net::IpAddr =
            params.config.listen.host.parse().map_err(|parse_err| {
                ApiServerCreateError::Ip(parse_err, params.config.listen.host.clone())
            })?;
        let listen_address = std::net::SocketAddr::new(listen_address, params.config.listen.port);

        let listener = std::net::TcpListener::bind(listen_address)?;
        let ssl_builder = match params.tls_mode {
            TlsMode::Enabled | TlsMode::Forced => daemons::ssl::create_ssl_acceptor_builder(
                &params.data_dir.join("etc/pki"),
                daemons::ssl::MutualAuthentication::Enabled,
                "apiserver",
            )
            .map(Some),
            TlsMode::Disabled => Ok(None),
        };
        let ssl_builder = match ssl_builder {
            Err(err) => match params.tls_mode {
                TlsMode::Forced => return Err(ApiServerCreateError::Ssl(err)),
                TlsMode::Enabled => {
                    log::warn!("failed to initialize TLS: {}", err);
                    None
                }
                TlsMode::Disabled => None,
            },

            Ok(b) => b,
        };

        let make_app = {
            let token_manager = params.token_manager.clone();
            let entity_loader = params.entity_loader.clone();
            let problem_loader = params.problem_loader.clone();
            let db_conn = params.db_conn.clone();
            let config = params.config.clone();
            let data_dir: std::sync::Arc<std::path::Path> = params.data_dir.into();

            move || {
                let token_mgr = token_manager.clone();
                let entity_loader = entity_loader.clone();
                let problem_loader = problem_loader.clone();
                let db_conn = db_conn.clone();
                let config = config.clone();
                let data_dir = data_dir.clone();

                let db_cx = crate::api::context::DbContext::create(db_conn.clone());
                let en_cx = crate::api::context::EntityContext::create(
                    entity_loader.clone(),
                    problem_loader.clone(),
                );
                let authorizer = create_authorizer(db_cx, en_cx);
                actix_web::App::new()
                    .app_data(secret_key::SecretKey(token_mgr.secret_key().into()))
                    .app_data(token_mgr)
                    .app_data(entity_loader)
                    .app_data(problem_loader)
                    .app_data(db_conn)
                    .app_data(authorizer)
                    .app_data(std::rc::Rc::new(config))
                    .app_data::<std::rc::Rc<std::path::Path>>((*data_dir).into())
                    .configure(api::misc::register_routes)
                    .configure(api::contests::register_routes)
                    .configure(api::runs::register_routes)
                    .configure(api::monitor::register_routes)
                    .configure(api::toolchains::register_routes)
                    .configure(api::auth::register_routes)
                    .configure(api::users::register_routes)
                    .route("/", actix_web::web::get().to(route_ping))
            }
        };

        let server = actix_web::HttpServer::new(make_app)
            .disable_signals()
            .on_connect(daemons::ssl::make_on_connect_hook());

        let server = match ssl_builder {
            Some(ssl_builder) => server.listen_openssl(listener, ssl_builder)?,
            None => server.listen(listener)?,
        };let server = if params.single_worker {
            server.workers(1)
        }else {server};

        let server = server.run();
        {
            let server = server.clone();
            tokio::task::spawn(async move {
                rx.recv().await;
                server.stop(true).await;
            });
        }
        startup_tx.send(()).ok();

        if let Err(serve_err) = server.await {
            eprintln!("Fatal error: {}", serve_err);
        }
        Ok(())
    }

    pub async fn create(params: ApiserverParams) -> Result<ApiServer, ApiServerCreateError> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (startup_tx, startup_rx) = tokio::sync::oneshot::channel();
        debug!("about to spawn apiserver task");
        // TODO: do not create second Tokio runtime
        tokio::task::spawn_blocking(move || {
            if let Err(err) = Self::serve(params, rx, startup_tx) {
                eprintln!("Startup error: {:#}", err);
            }
        });

        startup_rx.await.ok();

        Ok(ApiServer {
            shutdown: ShutdownHandle { chan: tx },
        })
    }

    pub fn get_shutdown_handle(&self) -> &ShutdownHandle {
        &self.shutdown
    }
}

fn create_authorizer(
    db_cx: crate::api::context::DbContext,
    en_cx: crate::api::context::EntityContext,
) -> crate::api::security::Authorizer {
    let mut builder = crate::api::security::Authorizer::builder();

    {
        let mut default_pipeline_builder = crate::api::security::Pipeline::builder();
        default_pipeline_builder.set_name("default".to_string());
        crate::api::security::rules::install(&mut default_pipeline_builder, db_cx, en_cx);
        builder.add_pipeline(default_pipeline_builder.build());
    }
    {
        let sudo_pipeline = crate::api::security::create_sudo_pipeline();
        builder.add_pipeline(sudo_pipeline);
    }

    builder.build()
}
