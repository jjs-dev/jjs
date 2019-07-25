use juniper::{GraphQLObject};

pub type ToolchainId = i32;
pub type RunId = i32;
pub type ProblemId = String;
pub type ContestId = String;

#[derive(GraphQLObject)]
pub struct InvokeStatus {
    pub kind: String,
    pub code: String,
}
#[derive(GraphQLObject)]
pub(crate) struct Run {
    pub id: RunId,
    pub toolchain_name: String,
    pub status: InvokeStatus,
    pub score: Option<i32>,
    pub problem: ProblemId,
}
