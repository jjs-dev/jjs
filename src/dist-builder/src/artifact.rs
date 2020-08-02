pub(crate) struct RustArtifact {
    pub(crate) package_name: String,
}

pub(crate) struct CmakeArtifact {
    pub(crate) package_name: String,
}

pub(crate) enum Artifact {
    Rust(RustArtifact),
    Cmake(CmakeArtifact),
}
