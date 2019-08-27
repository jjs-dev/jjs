fn main() {
    let schema = frontend_engine::ApiServer::get_schema();
    // TODO write to out_dir
    let mut out_path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    out_path = format!("{}/src/schema-gen.json", out_path);
    let cur_schema = std::fs::read_to_string(&out_path).ok().unwrap_or_else(String::new);
    if schema != cur_schema {
        std::fs::write(out_path, schema).unwrap();
    }
}
