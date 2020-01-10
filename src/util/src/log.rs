use slog::{o, Drain, Logger, SendSyncRefUnwindSafeDrain};
use std::sync::atomic::{AtomicBool, Ordering};

fn make_drain() -> Box<dyn SendSyncRefUnwindSafeDrain<Ok = (), Err = slog::Never>> {
    if std::env::var("JJS_SYSTEMD").is_ok() {
        Box::new(slog_journald::JournaldDrain.ignore_res())
    } else {
        Box::new(
            std::sync::Mutex::new(
                slog_term::CompactFormat::new(slog_term::TermDecorator::new().stderr().build())
                    .build(),
            )
            .fuse(),
        )
    }
}

pub fn setup() {
    static FLAG: AtomicBool = AtomicBool::new(false);
    if FLAG.swap(true, Ordering::SeqCst) {
        return;
    }
    let drain = make_drain();

    let logger = slog_envlogger::new(drain);
    let logger = std::sync::Mutex::new(logger);
    let logger = Logger::root(logger.fuse(), o!());
    let guard = slog_scope::set_global_logger(logger.clone());
    slog_stdlog::init().unwrap();
    std::mem::forget(guard);
}
