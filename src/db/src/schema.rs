mod invocation;
mod participation;

pub use invocation::InvocationState;
pub use participation::ParticipationPhase;

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

pub type RunId = i32;
pub type InvocationId = i32;
pub type UserId = uuid::Uuid;
pub type ProblemId = String;
pub type ContestId = String;
pub type ParticipationId = i32;

#[derive(Serialize, Deserialize, Debug, Clone, Queryable, PartialEq, Eq)]
pub struct Run {
    pub id: RunId,
    pub toolchain_id: String,
    pub problem_id: ProblemId,
    pub rejudge_id: i32,
    pub user_id: UserId,
    pub contest_id: ContestId,
}

#[derive(Insertable)]
#[table_name = "runs"]
pub struct NewRun {
    pub toolchain_id: String,
    pub problem_id: ProblemId,
    pub rejudge_id: i32,
    pub user_id: UserId,
    pub contest_id: ContestId,
}

#[derive(AsChangeset, Default)]
#[table_name = "runs"]
pub struct RunPatch {
    #[column_name = "rejudge_id"]
    pub rejudge_id: Option<i32>,
}

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

#[derive(Insertable, Queryable)]
#[table_name = "kv"]
pub(crate) struct KvPair {
    #[column_name = "name"]
    pub(crate) key: String,
    pub(crate) value: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct Participation {
    pub id: ParticipationId,
    pub user_id: UserId,
    pub contest_id: ContestId,
    pub(crate) phase: i16,
    pub(crate) virtual_contest_start_time: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable, Default)]
#[table_name = "participations"]
pub struct NewParticipation {
    pub user_id: UserId,
    pub contest_id: ContestId,
    pub(crate) phase: i16,
    pub(crate) virtual_contest_start_time: Option<chrono::NaiveDateTime>,
}

use diesel::sql_types::*;

include!("./schema_raw.rs");
