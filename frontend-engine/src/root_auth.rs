// this module is responsible for root user authentification strategies
// it provides tcp service, which provides some platform-specific authentification options
use crate::FrontendConfig;
use slog::{error, info, Logger};
use std::{
    mem,
    os::unix::{
        io::AsRawFd,
        net::{UnixListener, UnixStream},
    },
};

#[derive(Clone)]
pub struct Config {
    pub socket_path: String,
    //pub token_provider: Arc<dyn Fn() -> String + Send + Sync>,
}

fn handle_conn(logger: &Logger, fcfg: &FrontendConfig, mut conn: UnixStream) {
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
    info!(logger, "issuing root credentials");
    let token = crate::security::Token::new_root().serialize(&fcfg.secret);
    let message = format!("==={}===\n", token);
    conn.write_all(message.as_bytes()).ok();
}

fn server_loop(logger: Logger,sock: UnixListener, fcfg: FrontendConfig) {
    info!(logger, "starting unix local root login service");
    for conn in sock.incoming() {
        if let Ok(conn) = conn {
            handle_conn(&logger, &fcfg, conn)
        }
    }
}

fn do_start(logger: Logger, cfg: Config, fcfg: &FrontendConfig) {
    info!(logger, "binding login server at {}", &cfg.socket_path);
    let listener = match UnixListener::bind(&cfg.socket_path) {
        Ok(l) => l,
        Err(err) => {
            error!(logger, "couldn't bind unix socket server due to {:?}",  err; "err" => ?err);
            return;
        }
    };
    let fcfg = fcfg.clone();
    std::thread::spawn(move || {
        server_loop(logger, listener, fcfg);
    });
}

pub struct LocalAuthServer {}

impl LocalAuthServer {
    pub fn start(logger: Logger, cfg: Config, fcfg: &FrontendConfig) -> Self {
        do_start(logger, cfg, fcfg);
        LocalAuthServer {}
    }
}
