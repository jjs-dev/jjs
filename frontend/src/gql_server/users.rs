use super::{schema, Context, InternalError};
use crate::password;
use diesel::prelude::*;
use juniper::FieldResult;

pub(crate) fn create(
    ctx: &Context,
    login: String,
    password: String,
    groups: Vec<String>,
) -> FieldResult<schema::User> {
    use db::schema::users::dsl;

    let provided_password_hash = password::get_password_hash(&password);

    let new_user = db::schema::NewUser {
        username: login.clone(),
        password_hash: provided_password_hash,
        groups: groups.clone(),
    };

    let conn = ctx.pool.get().map_err(InternalError::from)?;

    let user: db::schema::User = diesel::insert_into(dsl::users)
        .values(&new_user)
        .get_result(&conn)
        .map_err(InternalError::from)?;

    Ok((&user).into())
}
