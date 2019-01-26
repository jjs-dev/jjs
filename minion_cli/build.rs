use chrono::prelude::*;
fn main() {
    let time: DateTime<Utc> = Utc::now();
    let time = time.to_string();
    println!("cargo:rustc-env=MINION_CLI_COMPILATION_TIME={}", time);
    println!("cargo:rerun-if-changed=no-such.txt");
}
