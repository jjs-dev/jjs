#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct Submission {
    pub id: i32,
    pub toolchain: String,
    pub status: String,
    pub status_kind: String,
    pub problem_name: String,
    pub score: i32,
    pub rejudge_id: i32,
}

#[derive(Insertable)]
#[table_name = "submissions"]
pub struct NewSubmission {
    pub toolchain_id: String,
    pub status_code: String,
    pub status_kind: String,
    pub problem_name: String,
    pub score: i32,
    pub rejudge_id: i32,
}

#[derive(AsChangeset, Default)]
#[table_name = "submissions"]
pub struct SubmissionPatch {
    pub status_code: Option<String>,
    pub status_kind: Option<String>,
    #[column_name = "score"]
    pub score: Option<i32>,
    #[column_name = "rejudge_id"]
    pub rejudge_id: Option<i32>,
}

#[derive(Queryable, Debug, Clone, Serialize, Deserialize)]
pub struct InvokationRequest {
    pub id: i32,
    pub submission_id: i32,
    pub invoke_revision: i32,
}

#[derive(Insertable)]
#[table_name = "invokation_requests"]
pub struct NewInvokationRequest {
    pub submission_id: i32,
    pub invoke_revision: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct User {
    pub id: uuid::Uuid,
    pub username: String,
    pub password_hash: String,
    pub groups: Vec<String>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub password_hash: String,
    pub groups: Vec<String>,
}

use diesel::sql_types::*;

include!("./schema_raw.rs");
