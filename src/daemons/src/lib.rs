pub mod cfg;
pub mod ssl;
pub mod wait;

use std::sync::atomic::AtomicBool;

/// Called by daemon component (e.g. `apiserver` and `invoker`), when it is
/// ready to serve Must be called once
pub fn daemon_notify_ready() {
    static CALLED: AtomicBool = AtomicBool::new(false);
    let was_called_before = CALLED.swap(true, std::sync::atomic::Ordering::Relaxed);
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
