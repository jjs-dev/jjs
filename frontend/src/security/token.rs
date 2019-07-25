use super::{
    acl_query::{AccessChecker, UserInfo},
    SecretKey,
};
use crate::security::AccessControlData;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use slog::{debug, Logger};

#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
    user_info: UserInfo,
}

impl Token {
    pub fn issue_for_user(user_name: &str, conn: &diesel::PgConnection) -> Token {
        Token {
            user_info: UserInfo::retrieve(user_name, conn),
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
        branca::encode(&ser, key, &nonce, 0).expect("Token encoding error")
    }
}

#[derive(Debug)]
pub enum TokenFromRequestError {
    Missing,
    BadFormat,
    Invalid,
    Branca(branca::errors::Error),
}

pub struct AccessCheckService<'a> {
    token: Token,
    access_control_data: &'a AccessControlData,
    logger: Logger,
}

impl<'a> AccessCheckService<'a> {
    pub fn to_access_checker(&'a self) -> AccessChecker<'a> {
        AccessChecker {
            root: &self.access_control_data.root,
            user_info: &self.token.user_info,
            logger: self.logger.clone(),
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for AccessCheckService<'a> {
    type Error = TokenFromRequestError;

    fn from_request(
        req: &'a Request<'r>,
    ) -> Outcome<AccessCheckService<'a>, TokenFromRequestError> {
        let key = req
            .guard::<rocket::State<SecretKey>>()
            .expect("Couldn't fetch SecretKey")
            .0
            .clone();

        let env = req
            .guard::<rocket::State<crate::config::Env>>()
            .expect("Couldn't fetch env");

        let logger = req
            .guard::<rocket::State<Logger>>()
            .expect("Couldn't fetch logger")
            .clone();

        let access_control_data = req
            .guard::<rocket::State<AccessControlData>>()
            .expect("Couldn't fetch access control data");

        let token_data = req.headers().get_one("X-Jjs-Auth");

        let inner = move || {
            let token_data = match token_data {
                Some(td) => td,
                None => return Err(TokenFromRequestError::Missing),
            };
            if env.is_dev() {
                if token_data.starts_with("dev_user:") {
                    let uid = &token_data[9..];
                    return Ok(Token::issue_for_virtual_user(uid.to_string(), vec![])); //TODO groups
                }
                if token_data.starts_with("dev_root") {
                    return Ok(Token::new_root());
                }
            }

            let decoded = match branca::decode(token_data, &key, 0 /* TODO: check TTL */) {
                Ok(dec) => dec,
                Err(br_err) => {
                    return Err(TokenFromRequestError::Branca(br_err));
                }
            };
            let de = serde_json::from_str(&decoded).expect("Couldn't deserialize Token");
            Ok(de)
        };
        let res = inner();
        match res {
            Ok(token) => Outcome::Success(AccessCheckService {
                token,
                access_control_data: access_control_data.inner(),
                logger: logger.clone(),
            }),
            Err(err) => {
                debug!(
                    logger,
                    "Token: returning Outcome::Failure due to error"; "error" => ?err
                );
                Outcome::Failure((rocket::http::Status::BadRequest, err))
            }
        }
    }
}
