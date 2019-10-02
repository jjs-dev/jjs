use super::prelude::*;
use crate::password;

// TODO allow creation without password
pub(super) fn create(
    ctx: &Context,
    login: String,
    password: String,
    groups: Vec<String>,
) -> ApiResult<schema::User> {
    // TODO transaction
    let cur_user = ctx.db.user_try_load_by_login(&login).internal(ctx)?;
    if let Some(..) = cur_user {
        return Err(ApiError::new(ctx, "UserAlreadyExists"));
    }

    let provided_password_hash = password::get_password_hash(&password);

    let new_user = db::schema::NewUser {
        username: login.clone(),
        password_hash: Some(provided_password_hash),
        groups: groups.clone(),
    };

    let user = ctx.db.user_new(new_user).internal(ctx)?;

    Ok((&user).into())
}
