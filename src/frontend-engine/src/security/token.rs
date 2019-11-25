use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInfo {
    pub(super) id: uuid::Uuid,
    /// TODO: name should have hierarchical type
    pub(super) name: String,
    pub(super) groups: Vec<String>,
}

/// Struct representing API session
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub(super) session_id: uuid::Uuid,
    pub(super) user_info: UserInfo,
}

impl Token {
    pub fn issue_for_virtual_user(id: uuid::Uuid, name: String, groups: Vec<String>) -> Token {
        Token {
            user_info: UserInfo { id, name, groups },
            session_id: uuid::Uuid::new_v4(),
        }
    }

    pub fn user_id(&self) -> uuid::Uuid {
        self.user_info.id
    }

    pub fn session_id(&self) -> uuid::Uuid {
        self.session_id
    }
}
