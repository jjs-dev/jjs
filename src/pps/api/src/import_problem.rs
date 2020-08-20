//! Import problem from some other format
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize)]
pub struct Request {
    /// this path specifies file or files that should be imported
    pub src_path: PathBuf,
    /// where to put generated problem source
    pub out_path: PathBuf,
    /// do not check that dest is empty
    pub force: bool,
}

#[derive(Serialize, Deserialize)]
pub enum Update {
    /// Contains one property of discovered problem.
    /// Each `property_name` will be reported at most once.
    Property {
        property_name: PropertyName,
        property_value: String,
    },
    /// Contains one warnings. May appear multiple times.
    Warning(String),
    /// Started importing checker
    ImportChecker,
    /// Started importing tests
    ImportTests,
    /// Finished importing tests. `count` tests imported.
    ImportTestsDone { count: usize },
    /// Started importing solutions
    ImportSolutions,
    /// Started importing solution with specific name
    ImportSolution(String),
    /// Valuer config is detected and will be imported
    ImportValuerConfig,
    /// Valuer config was not found, default will be used
    DefaultValuerConfig,
}

#[derive(Serialize, Deserialize)]
pub enum PropertyName {
    /// Value is time limit in milliseconds.
    TimeLimit,
    /// Value is memory limit in milliseconds.
    MemoryLimit,
    /// Value is printf-style pattern of input files.
    InputPathPattern,
    /// Value is printf-style pattern of output files.
    OutputPathPattern,
    /// Value is problem title.
    ProblemTitle,
}
