use super::prelude::*;
use crate::password;

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct User {
    // TODO use Uuid here when diesel supports uuidv8
    /// UUID of this user.
    pub id: String,
    pub login: String,
}

impl ApiObject for User {
    fn name() -> &'static str {
        "User"
    }
}

impl<'a> From<&'a db::schema::User> for User {
    fn from(user: &'a db::schema::User) -> User {
        User {
            id: user.id.to_hyphenated().to_string(),
            login: user.username.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct UserCreateParams {
    /// Login (must be unique)
    login: String,
    /// Password (no strength validation is performed)
    password: String,
    /// List of groups new user should belong to
    #[serde(default)]
    groups: Vec<String>,
}

impl ApiObject for UserCreateParams {
    fn name() -> &'static str {
        "UserCreateParams"
    }
}

// TODO allow creation without password
#[post("/users", data = "<p>")]
pub(crate) fn route_create(ctx: Context, mut p: Json<UserCreateParams>) -> ApiResult<Json<User>> {
    // TODO transaction
    if !p.groups.is_empty() {
        let access_checker = ctx.access();
        if !access_checker.is_sudo().internal(&ctx)? {
            return Err(ApiError::access_denied(&ctx));
        }
    }
    let cur_user = ctx.db().user_try_load_by_login(&p.login).internal(&ctx)?;
    if let Some(..) = cur_user {
        return Err(ApiError::new(&ctx, "UserAlreadyExists"));
    }

    let provided_password_hash = password::get_password_hash(&p.password);

    let new_user = db::schema::NewUser {
        username: std::mem::take(&mut p.login),
        password_hash: Some(provided_password_hash),
        groups: std::mem::take(&mut p.groups),
    };

    let user = ctx.db().user_new(new_user).internal(&ctx)?;

    Ok(Json((&user).into()))
}
