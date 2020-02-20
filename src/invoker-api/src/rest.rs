//! Defines REST api
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInvokeTask {
    /// Run source in base64 encofing
    pub run_source: Option<String>,
    /// Invocation outputs directory
    pub invocation_dir: PathBuf,
    /// Toolchain definition
    pub toolchain: cfg::Toolchain,
    /// Problem definition
    pub problem: cfg::Problem,
    /// Invocation id (will be preserved by invoker)
    pub invocation_id: uuid::Uuid,
}

impl std::fmt::Display for HttpInvokeTask {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "HttpInvokeTask(")?;
        write!(f, "problem = {}", self.problem.name)?;
        Ok(())
    }
}
