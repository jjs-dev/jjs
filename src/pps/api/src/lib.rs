//! All PPS apis.
//!
//! All paths are relative to workspace root.
pub mod compile_problem;
pub mod import_problem;

use rpc::Route;
use std::convert::Infallible;
pub struct CompileProblem(Infallible);

impl Route for CompileProblem {
    type Request = rpc::Unary<compile_problem::Request>;
    type Response = rpc::Streaming<compile_problem::Update, SimpleFinish>;

    const ENDPOINT: &'static str = "/problems/compile";
}

pub struct ImportProblem(Infallible);

impl Route for ImportProblem {
    type Request = rpc::Unary<import_problem::Request>;
    type Response = rpc::Streaming<import_problem::Update, SimpleFinish>;

    const ENDPOINT: &'static str = "/problems/import";
}

/// Contains possible error os success
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[must_use = "this is Result in fact"]
pub struct SimpleFinish(pub Result<(), StringError>);

impl From<anyhow::Result<()>> for SimpleFinish {
    fn from(r: anyhow::Result<()>) -> Self {
        Self(r.map_err(|e| StringError(format!("{:#}", e))))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct StringError(pub String);

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for StringError {}
