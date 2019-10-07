use slog::{o, Drain, Logger};
use std::sync::atomic::{AtomicBool, Ordering};
pub fn setup() {
    static FLAG: AtomicBool = AtomicBool::new(false);
    if FLAG.swap(true, Ordering::SeqCst) {
        return;
    }
    let drain =
        slog_term::CompactFormat::new(slog_term::TermDecorator::new().stderr().build()).build();

    let logger = slog_envlogger::new(drain);
    let logger = std::sync::Mutex::new(logger);
    let logger = Logger::root(logger.fuse(), o!()).into_erased();
    let guard = slog_scope::set_global_logger(logger.clone());
    slog_stdlog::init().unwrap();
    std::mem::forget(guard);
}
