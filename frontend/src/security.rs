use rocket::request::{FromRequest, Outcome, Request};

#[derive(Serialize, Deserialize)]
pub struct Token {
    pub user_id: String,
}

#[derive(Clone)]
pub struct SecretKey(pub Vec<u8>);

pub const AUTH_HEADER_NAME: &str = "X-JJS-Auth";

impl Token {
    pub fn create_for_user(user_id: String) -> Token {
        Token { user_id }
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
        let token_data = req.headers().get_one("X-Jjs-Auth");

        let st = rocket::http::Status::BadRequest;
        let token_data = match token_data {
            Some(td) => td,
            None => return Outcome::Failure((st, TokenFromRequestError::Missing)),
        };
        //FIXME check env
        if token_data.starts_with("dev:") {
            let uid = &token_data[4..];
            return Outcome::Success(Token {
                user_id: uid.to_string(),
            });
        }

        let decoded = match branca::decode(token_data, &key, 0 /*TODO: check TTL*/) {
            Ok(dec) => dec,
            Err(br_err) => return Outcome::Failure((st, TokenFromRequestError::Branca(br_err))),
        };
        let de = serde_json::from_str(&decoded).expect("Couldn't deserialize Token");
        Outcome::Success(de)
    }
}
