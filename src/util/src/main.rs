use util;

fn main() {
    util::log::setup();
    println!(
        "wait spec: {}",
        std::env::var("JJS_WAIT").unwrap_or_default()
    );
    util::wait::wait();
}
