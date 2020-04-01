use super::{token::UserInfo, Token};
use std::sync::Arc;

/// Token Manager - entity manipulating tokens
#[derive(Clone, Debug)]
pub struct TokenMgr {
    db: Arc<dyn db::DbConn>,
    secret_key: Arc<[u8]>,
}

static TOKEN_PREFIX_BRANCA: &str = "Branca::";
static TOKEN_PREFIX_DEV: &str = "Dev::";
static TOKEN_PREFIX_GUEST: &str = "Guest::";

#[derive(Debug, thiserror::Error)]
pub enum TokenMgrError {
    #[error("db error: {source}")]
    Db {
        #[from]
        source: db::Error,
    },
    #[error("user '{}' not exists", user)]
    UserMissing { user: String },
    #[error("token not provided")]
    TokenMissing,
    #[error("token buffer format is invalid")]
    BadFormat,
    #[error("token buffer kind is unknown")]
    UnknownKind,
    #[error("using token is denied")]
    Denied,
    #[error("branca error: {}", source)]
    Branca { source: branca::errors::Error },
}

impl TokenMgr {
    pub fn new(db: Arc<dyn db::DbConn>, secret_key: Arc<[u8]>) -> Self {
        Self { db, secret_key }
    }

    pub fn secret_key(&self) -> &[u8] {
        &*self.secret_key
    }

    // TODO: use custom errors
    pub async fn create_token(&self, username: &str) -> Result<Token, TokenMgrError> {
        let user_data =
            self.db
                .user_try_load_by_login(username).await?
                .ok_or(TokenMgrError::UserMissing {
                    user: username.to_string(),
                })?;
        Ok(Token {
            user_info: UserInfo {
                name: user_data.username,
                groups: user_data.groups,
                id: user_data.id,
            },
            session_id: uuid::Uuid::new_v4(),
        })
    }

    pub async fn create_guest_token(&self) -> Result<Token, TokenMgrError> {
        self.create_token("Global/Guest").await
    }

    pub async fn create_root_token(&self) -> Result<Token, TokenMgrError> {
        self.create_token("Global/Root").await
    }

    pub fn serialize(&self, token: &Token) -> String {
        use rand::Rng;
        let ser = serde_json::to_string(token).expect("couldn't serialize Token");
        let mut rand_gen = rand::thread_rng();
        let mut nonce = [0 as u8; 24];
        rand_gen.fill(&mut nonce);
        let branca_data = branca::encode(&ser, self.secret_key(), 0).expect("Token encoding error");
        format!("Branca::{}", branca_data)
    }

    pub async fn deserialize(&self, data: &[u8], allow_dev: bool) -> Result<Token, TokenMgrError> {
        let data = match std::str::from_utf8(data) {
            Ok(d) => d,
            Err(_) => return Err(TokenMgrError::BadFormat),
        };
        const TOKEN_SCHEME: &str = "Token ";
        if !data.starts_with(TOKEN_SCHEME) {
            return Err(TokenMgrError::BadFormat);
        }
        let data = &data[TOKEN_SCHEME.len()..];
        if data.starts_with(TOKEN_PREFIX_BRANCA) {
            let data = &data[TOKEN_PREFIX_BRANCA.len()..];
            let token_data = match branca::decode(data, self.secret_key(), 0) {
                Ok(s) => s,
                Err(err) => return Err(TokenMgrError::Branca { source: err }),
            };
            let res = serde_json::from_str(&token_data).expect("Token decoding error");
            return Ok(res);
        }
        if data.starts_with(TOKEN_PREFIX_DEV) {
            if allow_dev {
                let data = &data[TOKEN_PREFIX_DEV.len()..];
                if data == "root" {
                    return Ok(self.create_root_token().await?);
                }
                if data.starts_with("User:") {
                    let data = data.trim_start_matches("User:");
                    return Ok(self.create_token(data).await?);
                }
                return Err(TokenMgrError::BadFormat);
            } else {
                return Err(TokenMgrError::Denied);
            }
        }
        if data.starts_with(TOKEN_PREFIX_GUEST) {
            return Ok(self.create_guest_token().await?);
        }

        Err(TokenMgrError::UnknownKind)
    }
}
