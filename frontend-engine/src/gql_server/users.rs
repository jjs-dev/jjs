use super::prelude::*;
use crate::password;

pub(super) fn create(
    ctx: &Context,
    login: String,
    password: String,
    groups: Vec<String>,
) -> ApiResult<schema::User> {
    let provided_password_hash = password::get_password_hash(&password);

    let new_user = db::schema::NewUser {
        username: login.clone(),
        password_hash: provided_password_hash,
        groups: groups.clone(),
    };

    let user = ctx.db.user_new(new_user)?;

    Ok((&user).into())
}
