use super::prelude::*;

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct SimpleAuthParams {
    /// Login
    login: String,
    /// Password
    password: String,
}

impl ApiObject for SimpleAuthParams {
    fn name() -> &'static str {
        "SimpleAuthParams"
    }
}

/// Type that represents session
/// You shouldn't do any assumptions about this type representation
#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct SessionToken {
    /// Opaque string that represents session data
    /// On all subsequent requests, put this string as value of header `X-Jjs-Auth`
    pub data: String,

    /// in dev mode, contains session data in unencrypted form
    pub raw_data: Option<String>,
}

impl ApiObject for SessionToken {
    fn name() -> &'static str {
        "SessionToken"
    }
}

#[post("/auth/simple", data = "<p>")]
pub(crate) fn route_simple(
    ctx: Context,
    p: Json<SimpleAuthParams>,
) -> ApiResult<Json<SessionToken>> {
    let mut success = false;
    let mut reject_reason = "";
    if let Some(user) = ctx.db().user_try_load_by_login(&p.login).internal(&ctx)? {
        if let Some(password_hash) = user.password_hash {
            success = crate::password::check_password_hash(&p.password, &password_hash);
            if !success {
                reject_reason = "IncorrectPassword";
            }
        } else {
            reject_reason = "PasswordAuthNotAvailable";
        }
    } else {
        reject_reason = "UnknownUser";
    }
    if success {
        let token = ctx.token_mgr.create_token(&p.login).internal(&ctx)?;
        let buf = ctx.token_mgr.serialize(&token);
        let sess = SessionToken {
            data: buf,
            raw_data: None, //TODO
        };
        Ok(Json(sess))
    } else {
        let mut ext = ErrorExtension::new();
        ext.set_error_code(reject_reason);
        let err = ApiError {
            visible: true,
            extension: ext,
            cause: None,
            ctx,
        };
        Err(err)
    }
}
