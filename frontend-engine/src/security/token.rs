use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInfo {
    /// TODO: name should have hierarchical type
    pub(super) name: String,
    pub(super) groups: Vec<String>,
}

/// Struct representing API session
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub(super) user_info: UserInfo,
}

static TOKEN_PREFIX_BRANCA: &str = "Branca ";
static TOKEN_PREFIX_DEV: &str = "Dev ";
static TOKEN_PREFIX_GUEST: &str = "Guest";

impl Token {
    pub fn issue_for_virtual_user(name: String, groups: Vec<String>) -> Token {
        Token {
            user_info: UserInfo { name, groups },
        }
    }

    pub fn new_guest() -> Token {
        Token {
            user_info: UserInfo {
                name: "Global/Guest".to_string(),
                groups: vec![],
            },
        }
    }

    pub fn new_root() -> Token {
        Token {
            user_info: UserInfo {
                name: "Global/Root".to_string(),
                groups: vec!["Sudoers".to_string()],
            },
        }
    }

    pub fn serialize(&self, key: &[u8]) -> String {
        use rand::Rng;
        let ser = serde_json::to_string(self).expect("couldn't serialize Token");
        let mut rand_gen = rand::thread_rng();
        let mut nonce = [0 as u8; 24];
        rand_gen.fill(&mut nonce);
        branca::encode(&ser, key, 0).expect("Token encoding error")
    }

    pub fn deserialize(
        key: &[u8],
        data: &[u8],
        allow_dev: bool,
    ) -> Result<Self, TokenFromRequestError> {
        let data = match std::str::from_utf8(data) {
            Ok(d) => d,
            Err(_) => return Err(TokenFromRequestError::BadFormat),
        };
        if data.starts_with(TOKEN_PREFIX_BRANCA) {
            let data = data.trim_start_matches(TOKEN_PREFIX_BRANCA);
            let token_data = match branca::decode(data, key, 0) {
                Ok(s) => s,
                Err(err) => return Err(TokenFromRequestError::Branca(err)),
            };
            let res = serde_json::from_str(&token_data).expect("Token decoding error");
            return Ok(res);
        }
        if data.starts_with(TOKEN_PREFIX_DEV) {
            if allow_dev {
                let data = data.trim_start_matches(TOKEN_PREFIX_DEV);
                if data == "root" {
                    return Ok(Token::new_root());
                }
                return Err(TokenFromRequestError::BadFormat);
            } else {
                return Err(TokenFromRequestError::Denied);
            }
        }
        if data.starts_with(TOKEN_PREFIX_GUEST) {
            return Ok(Token::new_guest());
        }

        Err(TokenFromRequestError::UnknownKind)
    }
}

#[derive(Debug)]
pub enum TokenFromRequestError {
    Missing,
    BadFormat,
    Invalid,
    UnknownKind,
    Denied,
    Branca(branca::errors::Error),
}
