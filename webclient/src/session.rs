use rocket::{
    http::{Cookie, Cookies, Status},
    outcome::Outcome,
    request::FromRequest,
    Request,
};

#[derive(Serialize, Deserialize)]
pub struct Auth {
    pub username: String,
    pub api_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct SessionData {
    pub auth: Option<Auth>,
    pub id: String,
}

impl SessionData {
    pub fn new() -> SessionData {
        SessionData {
            auth: None,
            id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl Default for SessionData {
    fn default() -> Self {
        SessionData::new()
    }
}

pub struct Session<'a, 'r> {
    request: &'a Request<'r>,
    should_expose: bool,
    pub data: SessionData,
}

impl<'a, 'r> Session<'a, 'r> {
    fn new(request: &'a Request<'r>, should_expose: bool) -> Session<'a, 'r> {
        Session {
            data: SessionData::new(),
            request,
            should_expose,
        }
    }

    pub fn assign(&self, req: &mut Cookies, with_public_copy: bool) {
        let encoded = serde_json::to_string(&self.data).unwrap();
        req.add_private(Cookie::new("session", encoded.clone()));
        if with_public_copy {
            req.add(Cookie::new("ses_pub", encoded));
        }
    }

    pub fn clear(&mut self) {
        self.data = SessionData::new();
    }

    pub fn expose(&mut self) {
        self.request
            .cookies()
            .add_private(Cookie::new("expose", "true"))
    }

    fn is_exposed(&self) -> bool {
        self.should_expose
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Session<'a, 'r> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Session<'a, 'r>, (Status, ()), ()> {
        let mut cookies = request.cookies();

        let should_expose = cookies
            .get_private("expose")
            .unwrap_or_else(||Cookie::new("expose", "false"))
            .value()
            == "true";
        let session = match cookies.get_private("session") {
            Some(s) => s,
            None => {
                //println!("cookie is absent or corrupted");
                return Outcome::Success(Session::new(request, should_expose));
            }
        };

        let sess_data: SessionData = match serde_json::from_str(session.value()) {
            Ok(s) => s,
            Err(_err) => {
                //println!("couldn't parse cookie ({}): {:?}", session.value(), err);
                return Outcome::Success(Session::new(request, should_expose));
            }
        };

        Outcome::Success(Session {
            data: sess_data,
            request,
            should_expose,
        })
    }
}

impl<'a, 'r> Drop for Session<'a, 'r> {
    fn drop(&mut self) {
        self.assign(&mut self.request.cookies(), self.is_exposed())
    }
}
