use crate::security::{token::UserInfo, Token};
use snafu::Snafu;
use std::sync::Arc;

/// Token Manager - entity manipulating tokens
pub(crate) struct TokenMgr {
    db: Arc<dyn db::DbConn>,
}

#[derive(Debug, Snafu)]
pub enum TokenMgrError {
    #[snafu(display("db error: {}", source))]
    Db { source: db::Error },
    #[snafu(display("user not exists"))]
    UserMissing,
}

impl From<db::Error> for TokenMgrError {
    fn from(source: db::Error) -> Self {
        Self::Db { source }
    }
}

impl TokenMgr {
    pub fn new(db: Arc<dyn db::DbConn>) -> Self {
        Self { db }
    }

    // TODO: use custom errors
    pub fn create_token(&self, username: &str) -> Result<Token, TokenMgrError> {
        let user_data = self
            .db
            .user_try_load_by_login(username)?
            .ok_or(TokenMgrError::UserMissing)?;
        Ok(Token {
            user_info: UserInfo {
                name: user_data.username,
                groups: user_data.groups,
            },
        })
    }
}
