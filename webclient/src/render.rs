#[derive(Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub login: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum AuthInfoData {
    User(UserInfo),
    Guest,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthInfo {
    d: AuthInfoData,
    is_user: bool,
    is_guest: bool,
}

impl AuthInfo {
    fn new(d: AuthInfoData) -> AuthInfo {
        let mut res = AuthInfo {
            d: d.clone(),
            is_user: false,
            is_guest: false,
        };
        match &d {
            AuthInfoData::User(_) => {
                res.is_user = true;
            }
            AuthInfoData::Guest => {
                res.is_guest = true;
            }
        };

        res
    }
}

impl From<AuthInfoData> for AuthInfo {
    fn from(d: AuthInfoData) -> AuthInfo {
        AuthInfo::new(d)
    }
}

#[derive(Serialize, Deserialize)]
pub struct CommonRenderContext {
    pub jjs_version: String,
    pub auth: AuthInfo,
    pub debug_info: String,
}

impl CommonRenderContext {
    pub fn fill_with_session_data(&mut self, session_data: &crate::session::SessionData) {
        if let Some(ref auth) = session_data.auth {
            self.auth = AuthInfo::new(AuthInfoData::User(UserInfo {
                login: auth.username.clone(),
            }))
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DefaultRenderContext {
    pub common: CommonRenderContext,
}

impl DefaultRenderContext {
    pub fn default() -> DefaultRenderContext {
        let ctx = CommonRenderContext {
            jjs_version: env!("CARGO_PKG_VERSION").to_string(),
            auth: AuthInfoData::Guest.into(),
            debug_info: "".to_string(),
        };

        let mut ctx = DefaultRenderContext { common: ctx };
        ctx.set_debug_info();
        ctx
    }

    pub fn set_debug_info(&mut self) {
        self.common.debug_info = serde_json::to_string(&self).unwrap();
    }
}
