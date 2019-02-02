pub struct Submission {
    pub id: usize,
    pub toolchain: String,
}

pub enum SubmissionState {
    WaitInvoke,
    Invoke,
    Done,
    Error,
}