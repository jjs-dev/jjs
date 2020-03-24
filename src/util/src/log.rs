use std::sync::Once;

pub fn setup() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        env_logger::init();
    });
}
