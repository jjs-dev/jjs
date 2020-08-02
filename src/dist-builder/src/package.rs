//! Abstractions for package
#[derive(Debug)]
pub(crate) struct RustPackage {
    /// As in sources
    pub(crate) name: String,
    pub(crate) install_name: String,
    pub(crate) section: Section,
}

pub(crate) struct CmakePackage {
    pub(crate) name: String,
    pub(crate) section: Section,
}

#[derive(Debug)]
pub(crate) struct OtherPackage {
    /// As in sources
    pub(crate) name: String,
    pub(crate) section: Section,
}

/// Automatically enabled if user enabled specified section
#[derive(Debug)]
pub(crate) struct MetaPackage {
    pub(crate) name: String,
    pub(crate) section: Section,
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum Section {
    /// This component will run as part of JJS
    Daemon,
    /// This component is recommended for common scenario.
    Suggested,
    /// This component can be used to work with JJS
    Tool,
}

impl Section {
    pub(crate) const ALL: &'static [Section] =
        &[Section::Daemon, Section::Suggested, Section::Tool];

    /// Returns section name in plural.
    /// used for validating&parsing CLI args
    pub(crate) fn plural(self) -> &'static str {
        match self {
            Section::Daemon => "daemons",
            Section::Suggested => "suggested",
            Section::Tool => "tools",
        }
    }
}
