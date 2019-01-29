fn main() {
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_file = format!("./bindings.h");
    cbindgen::generate(crate_root)
        .expect("was unable to generate bindings")
        .write_to_file(out_file);
}