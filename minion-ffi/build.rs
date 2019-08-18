fn main() {
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_file = format!("../target/minion-ffi.h"); //TODO better path
    cbindgen::generate(crate_root)
        .expect("was unable to generate bindings")
        .write_to_file(out_file);
    std::fs::copy("./prepend.h", "../target/minion-ffi-prepend.h").unwrap();
}
