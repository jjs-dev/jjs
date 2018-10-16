use crate::base::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordAuthRequest {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordAuthSuccess {
    pub new_token: Token,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordAuthFail {

}

pub type PasswordAuthResult = Result<PasswordAuthSuccess, PasswordAuthFail>;
