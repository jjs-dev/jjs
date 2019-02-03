#[macro_use]
extern crate postgres_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate postgres;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Submission {
    pub id: usize,
    pub toolchain: String,
}

#[derive(ToSql, FromSql, Serialize, Deserialize, Debug, Clone)]
pub enum SubmissionState {
    WaitInvoke,
    Invoke,
    Done,
    Error,
}
