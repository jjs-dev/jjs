#[derive(Debug)]
pub enum StatusKind {
    //Rejected,
    /// e.g. Coding Style Violation
    CompilationError,
    //Partial,
    //Accepted,
    NotSet,
}

#[derive(Debug)]
pub struct Status {
    pub kind: StatusKind,
    pub code: String,
}
/*
#[derive(Debug)]
pub struct Limits {
    pub memory: u64,
    pub time: std::time::Duration,
}
*/
