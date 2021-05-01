pub(crate) struct RustArtifact {
    pub(crate) package_name: String,
    pub(crate) install_name: String,
}

pub(crate) enum Artifact {
    Rust(RustArtifact),
}
