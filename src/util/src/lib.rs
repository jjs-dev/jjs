pub mod cmd;
pub mod log;
pub mod wait;

use std::sync::atomic::AtomicBool;

/// Called by daemon component (e.g. `frontend` and `invoker`), when it is ready to serve
/// Must be called once
pub fn daemon_notify_ready() {
    static CALLED: AtomicBool = AtomicBool::new(false);
    let was_called_before = CALLED.swap(true, std::sync::atomic::Ordering::SeqCst);
    if was_called_before {
        panic!("daemon_notify_ready() called more than once");
    }
    if std::env::var("JJS_SD_NOTIFY").is_ok() {
        let success = libsystemd::daemon::notify(true, &[libsystemd::daemon::NotifyState::Ready])
            .expect("failed notify systemd");
        if !success {
            eprintln!("error: unable to notify systemd");
        }
    }
}

pub fn daemon_startup_sleep() {
    if let Ok(duration) = std::env::var("JJS_DEV_SLEEP") {
        let duration: u8 = duration.parse().expect("invalid sleep duration");
        let duration = std::time::Duration::from_secs(duration as _);
        println!("Sleeping for {} seconds", duration.as_secs());
        std::thread::sleep(duration);
        println!("sleep done");
    }
}
