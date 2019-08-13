use juniper::{GraphQLInputObject, GraphQLObject};
use uuid::Uuid;

pub type ToolchainId = i32;
pub type RunId = i32;
pub type ProblemId = String;
pub type ContestId = String;
pub type UserId = Uuid;

#[derive(GraphQLInputObject)]
pub struct InvokeStatus {
    pub kind: String,
    pub code: String,
}

#[derive(GraphQLObject)]
pub(crate) struct Run {
    pub id: RunId,
    pub toolchain_name: String,
    pub status: InvokeStatus,
    pub score: Option<i32>,
    pub problem: ProblemId,
}

#[derive(GraphQLObject)]
pub(crate) struct User {
    pub id: UserId,
    pub login: String,
}

impl<'a> From<&'a db::schema::User> for User {
    fn from(user: &'a db::schema::User) -> User {
        User {
            id: user.id,
            login: user.username.clone(),
        }
    }
}

#[derive(GraphQLObject)]
/// Type that represents session
/// You shouldn't do any assumptions about this type representation
pub(crate) struct SessionToken {
    /// Opaque string that represents session data
    /// On all subsequent requests, that this string as value of header `X-Jjs-Auth`
    pub data: String,

    /// in dev mode, contains session data in unencrypted form
    pub raw_data: Option<String>,
}
