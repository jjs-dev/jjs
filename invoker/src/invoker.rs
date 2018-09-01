pub enum StatusKind {
    Rejected, /// e.g. Coding Style Violation
    CompilationError,
    Partial,
    Accepted,
}

pub struct Status {
    pub kind: StatusKind,
    pub code: String,
}