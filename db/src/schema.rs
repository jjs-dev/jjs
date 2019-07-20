#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct Submission {
    id: i32,
    pub toolchain: String,
    pub state: SubmissionState,
    pub status: String,
    pub status_kind: String,
    pub problem_name: String,
    pub score: i32,
    pub rejudge_id: i32,
}

impl Submission {
    pub fn id(&self) -> u32 {
        self.id as u32
    }

    pub fn rejudge_id(&self) -> u32 {
        self.rejudge_id as u32
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
    pub score: i32,
    pub rejudge_id: i32,
}

#[derive(AsChangeset, Default)]
#[table_name = "submissions"]
pub struct SubmissionPatch {
    pub state: Option<SubmissionState>,
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

impl InvokationRequest {
    pub fn id(&self) -> u32 {
        self.id as u32
    }

    pub fn submission_id(&self) -> u32 {
        self.submission_id as u32
    }

    pub fn invoke_revision(&self) -> u32 {
        self.invoke_revision as u32
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Queryable)]
pub struct User {
    id: i32,
    pub username: String,
    pub password_hash: String,
    pub groups: Vec<String>,
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
    pub groups: Vec<String>,
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
