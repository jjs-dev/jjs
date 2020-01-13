mod invocation;

// use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub type RunId = i32;
pub type InvocationId = i32;
pub type UserId = uuid::Uuid;
pub type ProblemId = String;

#[derive(Serialize, Deserialize, Debug, Clone, Queryable, PartialEq, Eq)]
pub struct Run {
    pub id: RunId,
    pub toolchain_id: String,
    pub problem_id: ProblemId,
    pub rejudge_id: i32,
    pub user_id: UserId,
}

#[derive(Insertable)]
#[table_name = "runs"]
pub struct NewRun {
    pub toolchain_id: String,
    pub problem_id: ProblemId,
    pub rejudge_id: i32,
    pub user_id: UserId,
}

#[derive(AsChangeset, Default)]
#[table_name = "runs"]
pub struct RunPatch {
    #[column_name = "rejudge_id"]
    pub rejudge_id: Option<i32>,
}
pub use invocation::InvocationState;

#[derive(Queryable, QueryableByName, Debug, Clone, Serialize, Deserialize)]
#[table_name = "invocations"]
pub struct Invocation {
    pub id: InvocationId,
    pub run_id: RunId,
    pub(crate) invoke_task: Vec<u8>,
    pub(crate) state: i16,
    pub(crate) outcome: serde_json::Value,
}

#[derive(Insertable)]
#[table_name = "invocations"]
pub struct NewInvocation {
    pub run_id: RunId,
    pub(crate) invoke_task: Vec<u8>,
    pub(crate) state: i16,
    pub(crate) outcome: serde_json::Value,
}

#[derive(AsChangeset, Default)]
#[table_name = "invocations"]
pub struct InvocationPatch {
    #[column_name = "state"]
    pub(crate) state: Option<i16>,
    #[column_name = "outcome"]
    pub(crate) outcome: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Queryable, Insertable)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: Option<String>,
    pub groups: Vec<String>,
}

pub struct NewUser {
    pub username: String,
    pub password_hash: Option<String>,
    pub groups: Vec<String>,
}

use diesel::sql_types::*;

include!("./schema_raw.rs");
