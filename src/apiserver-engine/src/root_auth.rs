// this module is responsible for root user authentication strategies
// it implements tcp service, which provides some platform-specific authentication options
use crate::{ApiserverParams, TokenMgr};
use futures::future::FutureExt;
use log::{error, info};
use std::{ffi::c_void, mem, os::unix::io::AsRawFd, sync::Arc};
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    stream::StreamExt,
    sync::oneshot::Receiver,
};

#[derive(Clone)]
pub struct Config {
    pub socket_path: String,
}

async fn handle_conn(token_mgr: TokenMgr, mut conn: UnixStream) {
    let conn_handle = conn.as_raw_fd();
    let mut peer_cred: libc::ucred = unsafe { mem::zeroed() };
    let mut len = mem::size_of_val(&peer_cred) as u32;
    unsafe {
        if libc::getsockopt(
            conn_handle,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut peer_cred as *mut _ as *mut c_void,
            &mut len,
        ) == -1
        {
            return;
        }
    }
    let my_uid = unsafe { libc::getuid() };
    if my_uid != peer_cred.uid {
        conn.write_all(b"error: your uid doesn't match that of jjs\n")
            .await
            .ok();
        return;
    }
    info!("issuing root credentials");
    let token = match token_mgr.create_root_token().await {
        Ok(tok) => token_mgr.serialize(&tok),
        Err(err) => {
            eprintln!("Error when issuing root credentials: {}", err);
            conn.write_all(format!("Error: {:#}", err).as_bytes())
                .await
                .ok();
            return;
        }
    };
    let message = format!("{}\n", token);
    conn.write_all(message.as_bytes()).await.ok();
}

async fn server_loop(mut sock: UnixListener, token_mgr: TokenMgr) {
    info!("starting unix local root login service");

    while let Some(conn) = sock.next().await {
        if let Ok(conn) = conn {
            handle_conn(token_mgr.clone(), conn).await
        }
    }
}

async fn do_start(cfg: Config, as_cfg: Arc<ApiserverParams>) {
    info!("binding login server at {}", &cfg.socket_path);
    tokio::fs::remove_file(&cfg.socket_path).await.ok();
    let listener = match UnixListener::bind(&cfg.socket_path) {
        Ok(l) => l,
        Err(err) => {
            error!("couldn't bind unix socket server: {}", err);
            return;
        }
    };
    server_loop(listener, as_cfg.token_mgr.clone()).await;
}

pub async fn exec(cfg: Config, fcfg: Arc<ApiserverParams>, rx: Receiver<()>) {
    let socket_path = cfg.socket_path.clone();
    let fut = do_start(cfg, fcfg);
    futures::future::select(
        Box::pin(fut),
        rx.map(|res| res.expect("tx disconnected unexpectedly")),
    )
    .await;
    tokio::fs::remove_file(socket_path).await.ok();
}
