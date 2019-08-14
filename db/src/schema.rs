use serde::{Deserialize, Serialize};

pub type RunId = i32;
pub type InvocationRequestId = i32;
pub type UserId = uuid::Uuid;
pub type ProblemId = String;

#[derive(Serialize, Deserialize, Debug, Clone, Queryable, PartialEq, Eq)]
pub struct Run {
    pub id: RunId,
    pub toolchain_id: String,
    pub status_code: String,
    pub status_kind: String,
    pub problem_id: ProblemId,
    pub score: i32,
    pub rejudge_id: i32,
}

#[derive(Insertable)]
#[table_name = "runs"]
pub struct NewRun {
    pub toolchain_id: String,
    pub status_code: String,
    pub status_kind: String,
    pub problem_id: ProblemId,
    pub score: i32,
    pub rejudge_id: i32,
}

#[derive(AsChangeset, Default)]
#[table_name = "runs"]
pub struct RunPatch {
    pub status_code: Option<String>,
    pub status_kind: Option<String>,
    #[column_name = "score"]
    pub score: Option<i32>,
    #[column_name = "rejudge_id"]
    pub rejudge_id: Option<i32>,
}

#[derive(Queryable, Debug, Clone, Serialize, Deserialize)]
pub struct InvocationRequest {
    pub id: InvocationRequestId,
    pub run_id: i32,
    pub invoke_revision: i32,
}

#[derive(Insertable)]
#[table_name = "invocation_requests"]
pub struct NewInvocationRequest {
    pub run_id: RunId,
    pub invoke_revision: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Queryable, Insertable)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: String,
    pub groups: Vec<String>,
}

pub struct NewUser {
    pub username: String,
    pub password_hash: String,
    pub groups: Vec<String>,
}

use diesel::sql_types::*;

include!("./schema_raw.rs");
