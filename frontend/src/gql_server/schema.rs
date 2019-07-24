use juniper::GraphQLObject;

pub struct InvokeStatus {
    pub kind: String,
    pub code: String,
}

pub enum SubmissionState {
    Queue,
    Judge,
    Finish,
    Error,
}

#[derive(GraphQLObject)]
pub(crate) struct Submission {
    pub id: SubmissionId,
    pub toolchain_name: String,
    pub status: InvokeStatus,
    pub state: SubmissionState,
    pub score: Option<i32>,
    pub problem: ProblemCode,
}