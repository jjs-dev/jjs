use rocket::request::{FromRequest, Outcome, Request};

#[derive(Serialize, Deserialize, Debug)]
pub enum TokenKind {
    User,
    Root,
    Guest,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
    pub user_id: Option<String>,
    pub kind: TokenKind,
}

#[derive(Clone)]
pub struct SecretKey(pub Vec<u8>);

pub const AUTH_HEADER_NAME: &str = "X-JJS-Auth";

impl Token {
    pub fn new_for_user(user_id: String) -> Token {
        Token {
            user_id: Some(user_id),
            kind: TokenKind::User,
        }
    }

    pub fn new_guest() -> Token {
        Token {
            user_id: None,
            kind: TokenKind::Guest,
        }
    }

    pub fn new_root() -> Token {
        Token {
            user_id: None,
            kind: TokenKind::Root,
        }
    }

    pub fn serialize(&self, key: &[u8]) -> String {
        use rand::Rng;
        let ser = serde_json::to_string(self).expect("couldn't serialize Token");
        let mut rand_gen = rand::thread_rng();
        let mut nonce = [0 as u8; 24];
        rand_gen.fill(&mut nonce);
        //let nonce =
        //let timestamp = time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_millis() as u32;
        branca::encode(&ser, key, &nonce, 0).expect("Token encoding error")
    }

    pub fn is_root(&self) -> bool {
        match self.kind {
            TokenKind::Root => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum TokenFromRequestError {
    Missing,
    BadFormat,
    Invalid,
    Branca(branca::errors::Error),
}

impl<'a, 'r> FromRequest<'a, 'r> for Token {
    type Error = TokenFromRequestError;

    fn from_request(req: &'a Request<'r>) -> Outcome<Token, TokenFromRequestError> {
        let key = req
            .guard::<rocket::State<SecretKey>>()
            .expect("Couldn't fetch SecretKey")
            .0
            .clone();

        let env = req
            .guard::<rocket::State<crate::util::Env>>()
            .expect("Couldn't fetch env");

        let token_data = req.headers().get_one("X-Jjs-Auth");

        let inner = move || {
            let token_data = match token_data {
                Some(td) => td,
                None => return Err(TokenFromRequestError::Missing),
            };
            if env.is_dev() {
                if token_data.starts_with("dev_user:") {
                    let uid = &token_data[9..];
                    return Ok(Token {
                        user_id: Some(uid.to_string()),
                        kind: TokenKind::User,
                    });
                }
                if token_data.starts_with("dev_root") {
                    return Ok(Token::new_root());
                }
            }

            let decoded = match branca::decode(token_data, &key, 0 /*TODO: check TTL*/) {
                Ok(dec) => dec,
                Err(br_err) => return Err(TokenFromRequestError::Branca(br_err)),
            };
            let de = serde_json::from_str(&decoded).expect("Couldn't deserialize Token");
            Ok(de)
        };
        let res = inner();
        match res {
            Ok(tok) => Outcome::Success(tok),
            Err(err) => Outcome::Failure((rocket::http::Status::BadRequest, err)),
        }
    }
}
