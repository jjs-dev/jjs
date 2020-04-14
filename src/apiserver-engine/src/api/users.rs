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
async fn route_create(
    scx: SecurityContext,
    dcx: DbContext,
    mut p: Json<UserCreateParams>,
) -> ApiResult<Json<User>> {
    // TODO transaction
    if !p.groups.is_empty() {
        scx.access()
            .with_conditions(make_conditions![])
            .with_action(Action::Create)
            .with_resource_kind(ResourceKind::USERS)
            .authorize()
            .await?;
    }
    let cur_user = dcx.db().user_try_load_by_login(&p.login).await.internal()?;
    if let Some(..) = cur_user {
        return Err(ApiError::new("UserAlreadyExists"));
    }

    let provided_password_hash = password::get_password_hash(&p.password);

    let new_user = db::schema::NewUser {
        username: std::mem::take(&mut p.login),
        password_hash: Some(provided_password_hash),
        groups: std::mem::take(&mut p.groups),
    };

    let user = dcx.db().user_new(new_user).await.internal()?;

    Ok(Json((&user).into()))
}

pub(crate) fn register_routes(c: &mut web::ServiceConfig) {
    c.route("/users", web::post().to(route_create));
}
