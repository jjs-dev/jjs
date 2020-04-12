//! Defines Invoker API
//!
//! If you just want to use JJS, you should look at apiserver.
//! This API is desired for advanced use cases, such as integrating invoker
//! in custom system.
//! Authentication is performed using TLS Client
//! Authorization, using $JJS_DATA/etc/pki/ca.crt as root of trust.

mod verify;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::Context as _;
use log::error;
use std::path::PathBuf;

#[derive(Clone)]
struct State {
    task_source: crate::sources::BackgroundSourceHandle,
    shutdown_trigger: tokio::sync::mpsc::Sender<()>,
}

/// invoker.{crt,key} - authorize invoker
/// ca.crt - authorize requests
const REQUIRED_PATHS: &[&str] = &["invoker.crt", "invoker.key", "ca.crt"];

async fn route_ping() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("hello, authenticated world!")
}

async fn route_health() -> impl Responder {
    // TODO account for real health here
    let health = serde_json::json!({
        "summary": "Ok"
    });
    serde_json::to_string(&health)
}

async fn route_shutdown(state: web::Data<State>) -> impl Responder {
    log::info!("invoker api: got shutdown request");
    state
        .shutdown_trigger
        .clone()
        .send(())
        .await
        .map(|_| "shutdown requested successfully")
        .map_err(|send_error| {
            actix_web::error::InternalError::new(
                anyhow::anyhow!("channel error: {}", send_error),
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })
}

fn verify_client_certificate(
    openssl_validation_succeeded: bool,
    chain: &mut openssl::x509::X509StoreContextRef,
) -> bool {
    if !openssl_validation_succeeded {
        return false;
    }
    let cert = match chain.chain() {
        Some(c) => c,
        None => return false,
    };
    let cert = match cert.iter().next() {
        Some(c) => c,
        None => return false,
    };
    let subject_names = cert.subject_name();
    for name in subject_names.entries_by_nid(openssl::nid::Nid::COMMONNAME) {
        let data = name.data().as_slice();
        if data == b"root" {
            return true;
        }
    }
    false
}

#[actix_rt::main]
async fn exec(
    mut stop_token: tokio::sync::broadcast::Receiver<!>,
    bind_addr: std::net::SocketAddr,
    task_source: crate::sources::BackgroundSourceHandle,
    shutdown_trigger: tokio::sync::mpsc::Sender<()>,
    pki_base: PathBuf,
) -> anyhow::Result<()> {
    let state = State {
        task_source,
        shutdown_trigger,
    };
    let mut some_pki_files_missing = false;
    for &path in REQUIRED_PATHS {
        let p = pki_base.join(path);

        if !p.exists() {
            some_pki_files_missing = true;
            error!("Missing: {}", p.display());
        }
    }
    if some_pki_files_missing {
        return Ok(());
    }
    let mut ssl_builder =
        openssl::ssl::SslAcceptor::mozilla_modern(openssl::ssl::SslMethod::tls())?;
    ssl_builder
        .set_certificate_chain_file(pki_base.join("invoker.crt"))
        .context("failed to load certificate")?;
    ssl_builder
        .set_private_key_file(pki_base.join("invoker.key"), openssl::ssl::SslFiletype::PEM)?;

    let ca_certificate = tokio::fs::read(pki_base.join("ca.crt"))
        .await
        .context("failed to read CA certificate")?;

    let ca_certificate = openssl::x509::X509::from_pem(&ca_certificate)
        .context("CA certificate is not valid PEM")?;
    let mut client_store_builder = openssl::x509::store::X509StoreBuilder::new()?;
    client_store_builder
        .add_cert(ca_certificate)
        .context("unable to put CA certificate into certificate store")?;
    ssl_builder.set_verify_cert_store(client_store_builder.build())?;

    let verify_mode =
        openssl::ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT | openssl::ssl::SslVerifyMode::PEER;
    // this callback will verify CN
    ssl_builder.set_verify_callback(verify_mode, verify_client_certificate);

    // disallow legacy (and potentially insecure) TLS versions
    ssl_builder.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1_2))?;

    let srv = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(actix_web::middleware::Logger::default())
            .route("/", web::get().to(route_ping))
            .route("/health", web::get().to(route_health))
            .route("/state/shutdown", web::post().to(route_shutdown))
    })
    .workers(1)
    .disable_signals()
    .bind_openssl(bind_addr, ssl_builder)
    .context("unable to bind")?
    .run();
    loop {
        match stop_token.recv().await {
            Err(tokio::sync::broadcast::RecvError::Closed) => break,
            Err(tokio::sync::broadcast::RecvError::Lagged(_)) => unreachable!(),
            Ok(never) => match never {},
        }
    }
    srv.stop(false).await;

    Ok(())
}

pub async fn start(
    stop_token: tokio::sync::broadcast::Receiver<!>,
    bind_addr: std::net::SocketAddr,
    task_source: crate::sources::BackgroundSourceHandle,
    shutdown_trigger: tokio::sync::mpsc::Sender<()>,
    pki_base: PathBuf,
) -> Result<(), anyhow::Error> {
    tokio::task::spawn_blocking(move || {
        if let Err(err) = exec(
            stop_token,
            bind_addr,
            task_source,
            shutdown_trigger,
            pki_base,
        ) {
            eprintln!("Invoker api service: serve error: {:#}", err);
        }
    });
    Ok(())
}
