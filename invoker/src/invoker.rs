#[derive(Debug)]
pub enum StatusKind {
    Rejected,
     /// e.g. Coding Style Violation
    CompilationError,
    Partial,
    Accepted,
}

#[derive(Debug)]
pub struct Status {
    pub kind: StatusKind,
    pub code: String,
}