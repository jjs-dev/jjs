#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct Submission {
    id: i32,
    pub toolchain: String,
    pub state: SubmissionState,
    pub status: String,
    pub status_kind: String,
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
    pub status: String,
    pub status_kind: String,
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
