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
        if allow_dev && data.starts_with("dev_") {
            let data = data.trim_start_matches("dev_");
            if data == "root" {
                return Ok(Token::new_root());
            }
            return Err(TokenFromRequestError::BadFormat);
        }
        let token_data = match branca::decode(data, key, 0) {
            Ok(s) => s,
            Err(err) => return Err(TokenFromRequestError::Branca(err)),
        };
        let res = serde_json::from_str(&token_data).expect("Token decoding error");
        Ok(res)
    }
}

#[derive(Debug)]
pub enum TokenFromRequestError {
    Missing,
    BadFormat,
    Invalid,
    Branca(branca::errors::Error),
}
