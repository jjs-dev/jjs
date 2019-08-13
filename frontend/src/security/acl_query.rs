use super::access_control::{ContestRights, GlobalRights};
use acl::{AccessToken, Prefix};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub name: String,
    pub groups: Vec<String>,
}

impl UserInfo {
    pub(crate) fn retrieve(name: &str) -> UserInfo {
        // FIXME
        UserInfo {
            name: name.to_string(),
            groups: vec![],
        }
    }

    fn as_access_token(&self) -> AccessToken {
        AccessToken {
            name: &self.name,
            groups: &self.groups,
        }
    }
}

pub struct AccessChecker<'a> {
    pub logger: slog::Logger,
    pub root: &'a Prefix,
    pub user_info: &'a UserInfo,
}

impl<'a> AccessChecker<'a> {
    fn check(&self, path: &[&str], access: u64) -> bool {
        let res = acl::access(self.root, self.user_info.as_access_token(), path, access);
        slog::debug!(self.logger, "running access check"; "user" => ?self.user_info, "path" => ?path, "outcome" => ?res);

        res.ok() == Some(access)
    }

    pub fn can_submit(&self) -> bool {
        let path = &["Contest", "CommonRights"];
        let desired_access = ContestRights::SUBMIT;
        self.check(path, desired_access.bits())
    }

    pub fn can_create_users(&self) -> bool {
        self.check(&["CommonRights"], (GlobalRights::MANAGE_USERS).bits())
    }

    pub fn can_view_contest(&self) -> bool {
        self.check(&["Contest", "CommonRights"], (ContestRights::VIEW).bits())
    }

    pub fn can_manage_submissions(&self) -> bool {
        self.check(&["Contest", "CommonRights"], (ContestRights::JUDGE).bits())
    }
}
