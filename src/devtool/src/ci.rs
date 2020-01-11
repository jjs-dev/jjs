use std::env::var;

#[derive(PartialEq, Eq, Clone, Debug)]
enum CheckJobType {
    EndToEnd,
    __Other,
}

impl CheckJobType {
    fn detect() -> Option<CheckJobType> {
        std::env::var("JOB")
            .ok()
            .and_then(|name| match name.as_str() {
                "e2e" => Some(CheckJobType::EndToEnd),
                _ => panic!("unknown job name: {}", name),
            })
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
enum BuildType {
    /// not a CI build
    NotCi,
    /// PR build,`bors try` or `bors r+`
    Check { ty: CheckJobType, privileged: bool },
    /// we are on master, want to build something special
    Deploy(DeployKind),
}

#[derive(Eq, PartialEq, Clone, Debug, Copy)]
pub enum DeployKind {
    Docker,
    Man,
    Deb,
}

impl DeployKind {
    fn detect() -> DeployKind {
        let e = var("JJS_DT_DEPLOY").expect("JJS_DT_DEPLOY missing");
        match e.as_str() {
            "docker" => DeployKind::Docker,
            "man" => DeployKind::Man,
            "deb" => DeployKind::Deb,
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

    pub fn is_pr_e2e(&self) -> bool {
        match self.ty {
            BuildType::Check {
                ty: CheckJobType::EndToEnd,
                ..
            } => true,
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

    let workflow_name = var("GITHUB_WORKFLOW").expect("GITHUB_WORKFLOW not exists");

    let branch_name = match extract_branch_name(&commit_ref) {
        Some(nam) => nam,
        None => panic!("Failed to parse commit ref: {}", &commit_ref),
    };
    if workflow_name == "deploy" {
        return BuildType::Deploy(DeployKind::detect());
    }
    let job_ty = CheckJobType::detect().expect("failed to detech check job");
    let privileged = match branch_name {
        "trying" | "staging" | "master" => true,
        _ => false,
    };
    BuildType::Check {
        ty: job_ty,
        privileged,
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
