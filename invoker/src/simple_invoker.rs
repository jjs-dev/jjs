//! implements very simple logic
//! if submission compiles, it's considered to be Accepted
//! else it gets Compilation Error
use ::object;
use ::invoker;

pub fn judge(submission: object::Submission) -> invoker::Status {


    invoker::Status {
        kind: invoker::StatusKind::Accepted,
        code: "OK".to_owned(),
    }
}