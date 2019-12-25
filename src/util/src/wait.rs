use std::{
    path::{Path, PathBuf},
    time::Duration,
};

type PollResult = std::io::Result<bool>;

fn poll_file(path: &Path) -> PollResult {
    match std::fs::File::open(path) {
        Ok(_file) => Ok(true),
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => Ok(false),
            _ => Err(err),
        },
    }
}

fn poll_tcp(addr: &std::net::SocketAddr) -> PollResult {
    match std::net::TcpStream::connect(addr) {
        Ok(_sock) => Ok(true),
        Err(_err) => {
            // TODO check errors
            Ok(false)
        }
    }
}

enum WaitItem {
    File(PathBuf),
    Tcp(std::net::SocketAddr),
}

impl WaitItem {
    fn poll(&self) -> PollResult {
        match self {
            WaitItem::File(path) => poll_file(path),
            WaitItem::Tcp(addr) => poll_tcp(addr),
        }
    }
}

impl std::fmt::Display for WaitItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "WaitCondition(")?;
        match self {
            WaitItem::File(path) => {
                write!(f, "file {} exists", path.display())?;
            }
            WaitItem::Tcp(addr) => {
                write!(f, "address {} reachable", addr)?;
            }
        }
        write!(f, ")")
    }
}

fn do_poll(items: &[WaitItem]) {
    let mut active = vec![0; items.len()];
    for (i, item) in active.iter_mut().enumerate() {
        *item = i;
    }
    let timeout = Duration::from_secs(90);
    let deadline = std::time::Instant::now() + timeout;
    while !active.is_empty() {
        log::debug!("Running poll attempt. {} remaining", active.len());
        let mut new_active = Vec::new();
        for &i in &active {
            let wait_item = &items[i];
            log::debug!("Polling #{}: {}", i + 1, wait_item);
            match wait_item.poll() {
                Ok(true) => {
                    log::debug!("#{} done", i + 1);
                }
                Ok(false) => {
                    new_active.push(i);
                }
                Err(err) => {
                    log::error!("wait #{} ({}) failed: {}", i + 1, wait_item, err);
                    std::process::exit(1);
                }
            }
        }
        std::thread::sleep(Duration::from_secs(3));
        if std::time::Instant::now() > deadline {
            log::error!("Wait timed out: {} waits outstanding", new_active.len());
            std::process::exit(1);
        }
        active = new_active;
    }
}

pub fn wait() {
    if let Ok(spec) = std::env::var("JJS_WAIT") {
        let items = spec.split(';');
        let mut waits = Vec::new();
        for item in items {
            if item.is_empty() {
                continue;
            }
            let item_url = match url::Url::parse(item) {
                Ok(u) => u,
                Err(parse_err) => {
                    eprintln!("failed parse wait spec {}: {}", item, parse_err);
                    std::process::exit(1);
                }
            };
            let new_wait = match item_url.scheme() {
                "file" => WaitItem::File(PathBuf::from(item_url.path())),
                "tcp" => {
                    let addr = match item_url.socket_addrs(|| None) {
                        Ok(addr) => addr
                            .into_iter()
                            .next()
                            .expect("empty socket addrs resolved"),
                        Err(err) => {
                            eprintln!("failed to resolve {}: {}", item_url, err);
                            std::process::exit(1);
                        }
                    };
                    WaitItem::Tcp(addr)
                }
                other => {
                    eprintln!("Unknown URL scheme: {}", other);
                    std::process::exit(1);
                }
            };
            log::debug!("wait item: {}", &new_wait);
            waits.push(new_wait);
        }
        log::info!("waiting {} conditions", waits.len());
        do_poll(&waits);
    }
}
