#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct Submission {
    id: i32,
    pub toolchain: String,
    pub state: SubmissionState,
    pub status: String,
    pub status_kind: String,
    pub problem_name: String,
    pub judge_revision: i32,
}

impl Submission {
    pub fn id(&self) -> u32 {
        self.id as u32
    }
}

#[derive(Insertable)]
#[table_name = "submissions"]
pub struct NewSubmission {
    pub toolchain_id: String,
    pub state: SubmissionState,
    pub status_code: String,
    pub status_kind: String,
    pub problem_name: String,
    pub judge_revision: i32,
}

#[derive(AsChangeset, Default)]
#[table_name = "submissions"]
pub struct SubmissionPatch {
    pub state: Option<SubmissionState>,
    pub status_code: Option<String>,
    pub status_kind: Option<String>,
    pub judge_revision: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct User {
    id: i32,
    pub username: String,
    pub password_hash: String,
}

impl User {
    pub fn id(&self) -> u32 {
        self.id as u32
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub password_hash: String,
}

#[derive(DbEnum, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[DieselType = "Submission_state"]
#[PgType = "submission_state"]
pub enum SubmissionState {
    WaitInvoke,
    Invoke,
    Done,
    Error,
}

use diesel::sql_types::*;

include!("./schema_raw.rs");
