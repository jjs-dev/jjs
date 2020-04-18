mod invocation;
mod participation;
mod run;
mod user;

pub use invocation::InvocationState;
pub use participation::ParticipationPhase;

use serde::{Deserialize, Serialize};

pub type RunId = i32;
pub type InvocationId = i32;
pub type UserId = uuid::Uuid;
pub type ProblemId = String;
pub type ContestId = String;
pub type ParticipationId = i32;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub id: RunId,
    pub toolchain_id: String,
    pub problem_id: ProblemId,
    pub rejudge_id: i32,
    pub user_id: UserId,
    pub contest_id: ContestId,
}

pub struct NewRun {
    pub toolchain_id: String,
    pub problem_id: ProblemId,
    pub rejudge_id: i32,
    pub user_id: UserId,
    pub contest_id: ContestId,
}

#[derive(Default)]
pub struct RunPatch {
    pub rejudge_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invocation {
    pub id: InvocationId,
    pub run_id: RunId,
    pub(crate) invoke_task: Vec<u8>,
    pub(crate) state: i16,
    pub(crate) outcome: serde_json::Value,
}

pub struct NewInvocation {
    pub run_id: RunId,
    pub(crate) invoke_task: Vec<u8>,
    pub(crate) state: i16,
    pub(crate) outcome: serde_json::Value,
}

#[derive(Default)]
pub struct InvocationPatch {
    pub(crate) state: Option<i16>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Participation {
    pub id: ParticipationId,
    pub user_id: UserId,
    pub contest_id: ContestId,
    pub(crate) phase: i16,
    pub(crate) virtual_contest_start_time: Option<chrono::NaiveDateTime>,
}

#[derive(Default)]
pub struct NewParticipation {
    pub user_id: UserId,
    pub contest_id: ContestId,
    pub(crate) phase: i16,
    pub(crate) virtual_contest_start_time: Option<chrono::NaiveDateTime>,
}
