//! Defines Invoker API
//!
//! If you just want to use JJS, you should look at apiserver.
//! This API is desired for advanced use cases, such as integrating invoker
//! in custom system.

use crate::controller::JudgeRequestAndCallbacks;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::Context as _;
use tracing::instrument;

#[derive(Clone)]
struct State {
    task_tx: async_channel::Sender<JudgeRequestAndCallbacks>,
    cancel_token: tokio::sync::CancellationToken,
}

async fn route_ping() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("hello, world!")
}

async fn route_ready() -> impl Responder {
    ""
}

async fn route_shutdown(state: web::Data<State>) -> impl Responder {
    tracing::info!("invoker api: got shutdown request");
    state.cancel_token.cancel();
    "cancellation triggered"
}

#[actix_rt::main]
#[instrument(skip(task_tx))]
async fn exec(
    cancel_token: tokio::sync::CancellationToken,
    bind_addr: std::net::SocketAddr,
    task_tx: async_channel::Sender<JudgeRequestAndCallbacks>,
) -> anyhow::Result<()> {
    let state = State {
        task_tx,
        cancel_token: cancel_token.clone(),
    };

    let srv = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(actix_web::middleware::Logger::default())
            .route("/", web::get().to(route_ping))
            .route("/ready", web::get().to(route_ready))
            .route("/state/shutdown", web::post().to(route_shutdown))
    })
    .workers(1)
    .disable_signals()
    .bind(bind_addr)
    .context("unable to bind")?
    .run();
    cancel_token.cancelled().await;
    srv.stop(false).await;

    Ok(())
}

pub async fn start(
    cancel_token: tokio::sync::CancellationToken,
    bind_addr: std::net::SocketAddr,
    task_tx: async_channel::Sender<JudgeRequestAndCallbacks>,
) -> Result<(), anyhow::Error> {
    tokio::task::spawn_blocking(move || {
        if let Err(err) = exec(cancel_token, bind_addr, task_tx) {
            eprintln!("Invoker api service: serve error: {:#}", err);
        }
    });
    Ok(())
}
