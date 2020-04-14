use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInfo {
    pub id: uuid::Uuid,
    /// TODO: name should have hierarchical type
    pub name: String,
    pub groups: Vec<String>,
}

/// Struct representing API session
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub session_id: uuid::Uuid,
    pub user_info: UserInfo,
}

impl Token {
    pub fn issue_for_virtual_user(id: uuid::Uuid, name: String, groups: Vec<String>) -> Token {
        Token {
            user_info: UserInfo { id, name, groups },
            session_id: uuid::Uuid::new_v4(),
        }
    }
}
