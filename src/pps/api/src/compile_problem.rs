//! Specifies that problem contained in workspace
//! should be compiled
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Request {
    /// Path to problem source directory
    pub problem_path: PathBuf,
    /// Where to put compiled package
    pub out_path: PathBuf,
    /// Ignore existing files in out_path
    pub force: bool,
}

#[derive(Serialize, Deserialize)]
pub enum Update {
    /// Contains some warnings that should be displayed to used.
    /// Appears at most once.
    Warnings(Vec<String>),
    /// Solution with given name is being built
    BuildSolution(String),
    /// Test generator with given name is being built
    BuildTestgen(String),
    /// Checker building started
    BuildChecker,
    /// Test generation started. `count` tests will be processed.
    /// Appears at most once before `GenerateTest` updates.
    GenerateTests { count: usize },
    /// Test `test_id` is being generated. Total test count is `count`.
    /// `test_id`s are in range 1..=`count`. It is gu
    GenerateTest { test_id: usize },
    /// Valuer config is being copied
    CopyValuerConfig,
}
