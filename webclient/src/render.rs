#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub login: String,
}

#[derive(Serialize, Deserialize)]
pub enum AuthInfo {
    User(UserInfo),

    ///unused
    ///must be true
    Guest(bool),
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
            self.auth = AuthInfo::User(UserInfo {
                login: auth.username.clone(),
            })
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
            auth: AuthInfo::Guest(true),
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
