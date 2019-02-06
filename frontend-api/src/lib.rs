#[macro_use]
extern crate serde_derive;

//auth
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub buf: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AccessErrorKind {
    IncorrectToken,
    AccessDenied,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessError {
    pub kind: AccessErrorKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleAuthParams {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SimpleAuthErrorKind {
    UnknownLogin,
    IncorrectPassword,
    NotSuitable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleAuthError {
    pub kind: SimpleAuthErrorKind,
}

//submissions
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitDeclaration {
    pub toolchain: String,
    pub code: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmitErrorKind {
    UnknownToolchain,
    ContestIsOver,
    PermissionDenied,
    SizeLimitExceeded,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitError {
    pub kind: SubmitErrorKind,
}

pub type SubmissionId = u32;

///This traits serve for documentation-only purposes
pub trait Frontend {
    ///POST /auth/anonymous
    ///POST /submissions/send
    fn submissions_send(sd: SubmitDeclaration) -> Result<SubmissionId, SubmitError>;
}
