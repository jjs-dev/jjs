fn main() {
    let build = std::env::var("JJS_BUILD_DIR").unwrap();
    let src = std::env::var("JJS_SRC_DIR").unwrap();
    let manifest_path = format!("{}/jjs-build-config.json", &build);
    let manifest = std::fs::read_to_string(manifest_path).unwrap();
    let manifest: deploy::cfg::Config = serde_json::from_str(&manifest).unwrap();
    let params = deploy::Params {
        cfg: manifest.clone(),
        src,
        build,
        sysroot: manifest.prefix.clone().unwrap_or_else(|| "/tmp/jjs-build-res-sysroot".to_string())
    };
    deploy::package(&params);
}
