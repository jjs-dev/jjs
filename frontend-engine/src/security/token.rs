use serde::{Deserialize, Serialize};

// TODO
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInfo {
    name: String,
    groups: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    user_info: UserInfo,
}

impl Token {
    pub fn issue_for_user(user_name: &str) -> Token {
        Token {
            user_info: UserInfo {
                name: user_name.to_string(),
                groups: Vec::new(),
            },
        }
    }

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
}

#[derive(Debug)]
pub enum TokenFromRequestError {
    Missing,
    BadFormat,
    Invalid,
    Branca(branca::errors::Error),
}
