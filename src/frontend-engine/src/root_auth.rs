// this module is responsible for root user authentification strategies
// it provides tcp service, which provides some platform-specific authentification options
use crate::{FrontendParams, TokenMgr};
use slog_scope::{error, info};
use std::{
    mem,
    os::unix::{
        io::AsRawFd,
        net::{UnixListener, UnixStream},
    },
    sync::Arc,
};

#[derive(Clone)]
pub struct Config {
    pub socket_path: String,
}

fn handle_conn(token_mgr: &TokenMgr, mut conn: UnixStream) {
    use std::{ffi::c_void, io::Write};
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
            .ok();
        return;
    }
    info!("issuing root credentials");
    let token = match token_mgr.create_root_token() {
        Ok(tok) => token_mgr.serialize(&tok),
        Err(err) => {
            eprintln!("Error when issuing root credentials: {}", err);
            conn.write_all(format!("Error: {:#}", err).as_bytes()).ok();
            return;
        }
    };
    let message = format!("{}\n", token);
    conn.write_all(message.as_bytes()).ok();
}

fn server_loop(sock: UnixListener, token_mgr: &TokenMgr) {
    info!("starting unix local root login service");
    for conn in sock.incoming() {
        if let Ok(conn) = conn {
            handle_conn(token_mgr, conn)
        }
    }
}

fn do_start(cfg: Config, fcfg: Arc<FrontendParams>) {
    info!("binding login server at {}", &cfg.socket_path);
    std::fs::remove_file(&cfg.socket_path).ok();
    let listener = match UnixListener::bind(&cfg.socket_path) {
        Ok(l) => l,
        Err(err) => {
            error!("couldn't bind unix socket server due to {:?}",  err; "err" => ?err);
            return;
        }
    };
    std::thread::spawn(move || {
        server_loop(listener, &fcfg.token_mgr);
    });
}

pub struct LocalAuthServer {}

impl LocalAuthServer {
    pub fn start(cfg: Config, fcfg: Arc<FrontendParams>) -> Self {
        do_start(cfg, fcfg);
        LocalAuthServer {}
    }
}
