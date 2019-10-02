use std::path::PathBuf;

fn main() {
    let build = std::env::var_os("JJS_BUILD_DIR").unwrap();
    let build = PathBuf::from(build);
    let src = std::env::var_os("JJS_SRC_DIR").unwrap();
    let src = PathBuf::from(src);
    let manifest_path = build.join("jjs-build-config.json");
    let manifest = std::fs::read_to_string(manifest_path).unwrap();
    let manifest: deploy::cfg::Config = serde_json::from_str(&manifest).unwrap();
    let params = deploy::Params {
        cfg: manifest.clone(),
        src,
        build: build.clone(),
        sysroot: manifest.prefix.clone().unwrap_or_else(|| {
            let path = build.join("jjs-build-res-sysroot");
            deploy::util::make_empty(&path).unwrap();
            path
        }),
    };
    deploy::package(&params);
}
