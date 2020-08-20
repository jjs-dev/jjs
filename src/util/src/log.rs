use std::sync::Once;

pub fn setup() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .with_ansi(false)
            // TODO allow customization
            .without_time()
            .with_writer(std::io::stderr)
            .init();
    });
}
