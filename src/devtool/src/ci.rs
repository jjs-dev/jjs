use std::env::var;

#[derive(PartialEq, Eq, Clone, Debug)]
enum BuildType {
    /// not a CI build
    NotCi,
    /// regular PR or push build
    Pr,
    /// `bors try` or `bors r+`
    Bors,
    /// we are on master, want to build something special
    Deploy(DeployKind),
}

#[derive(Eq, PartialEq, Clone, Debug, Copy)]
pub enum DeployKind {
    Docker,
    Man,
}

impl DeployKind {
    fn detect() -> DeployKind {
        let e = var("JJS_DT_DEPLOY").expect("JJS_DT_DEPLOY missing");
        match e.as_str() {
            "docker" => DeployKind::Docker,
            "man" => DeployKind::Man,
            _ => unreachable!("unknown deploy kind: {}", &e),
        }
    }
}

#[derive(Clone)]
pub struct BuildInfo {
    ty: BuildType,
}

impl BuildInfo {
    pub fn is_deploy(&self) -> bool {
        match self.ty {
            BuildType::Deploy(_) => true,
            _ => false,
        }
    }

    pub fn deploy_info(&self) -> Option<DeployKind> {
        match self.ty {
            BuildType::Deploy(dk) => Some(dk),
            _ => None,
        }
    }

    pub fn is_not_ci(&self) -> bool {
        self.ty == BuildType::NotCi
    }

    pub fn is_ci(&self) -> bool {
        !self.is_not_ci()
    }
}

fn extract_branch_name(commit_ref: &str) -> Option<&str> {
    for &pat in &["refs/heads/", "refs/pull/"] {
        if commit_ref.starts_with(pat) {
            return Some(&commit_ref[pat.len()..]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_works_for_branch() {
        assert_eq!(extract_branch_name("refs/heads/master"), Some("master"));
        assert_eq!(extract_branch_name("refs/heads/br-an.ch"), Some("br-an.ch"));
    }

    #[test]
    fn test_works_for_pr() {
        assert_eq!(extract_branch_name("refs/pull/master"), Some("master"));
        assert_eq!(extract_branch_name("refs/pull/br-an.ch"), Some("br-an.ch"));
    }

    #[test]
    fn test_returns_none_for_wrong_inputs() {
        assert_eq!(extract_branch_name("master"), None);
        assert_eq!(extract_branch_name("refs/holy-cow"), None);
        assert_eq!(extract_branch_name("refs/null/xxx"), None);
    }

    #[test]
    fn test_strips_prefix_only_once() {
        assert_eq!(
            extract_branch_name("refs/heads/refs/heads/m"),
            Some("refs/heads/m")
        );
        assert_eq!(
            extract_branch_name("refs/pull/refs/pull/n"),
            Some("refs/pull/n")
        );
    }
}

fn do_detect_build_type() -> BuildType {
    if var("CI").is_err() {
        return BuildType::NotCi;
    }
    let commit_ref = var("GITHUB_REF").expect("GITHUB_REF not exists");

    let branch_name = match extract_branch_name(&commit_ref) {
        Some(nam) => nam,
        None => panic!("Failed to parse commit ref: {}", &commit_ref),
    };
    match branch_name {
        "trying" | "staging" => BuildType::Bors,
        "master" => BuildType::Deploy(DeployKind::detect()),
        _ => BuildType::Pr,
    }
}

fn do_detect_build_info() -> BuildInfo {
    BuildInfo {
        ty: do_detect_build_type(),
    }
}

lazy_static::lazy_static! {
    static ref BUILD_INFO: BuildInfo = do_detect_build_info();
}
pub fn detect_build_type() -> BuildInfo {
    (*BUILD_INFO).clone()
}
