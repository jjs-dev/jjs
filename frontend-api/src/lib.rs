#[macro_use]
extern crate serde_derive;

/// Represents errors, which can happen in (almost) each method.
#[derive(Debug, Serialize, Deserialize)]
pub enum CommonError {
    AccessDenied,
    InternalError,
    AuthTokenFault,
}

// some typedefs
pub type ToolchainId = u32;
pub type SubmissionId = u32;

// auth
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub buf: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleAuthParams {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SimpleAuthError {
    UnknownLogin,
    IncorrectPassword,
    NotSuitable,
    Common(CommonError),
}

// submissions
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitDeclaration {
    pub toolchain: ToolchainId,
    pub code: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmitError {
    UnknownToolchain,
    ContestIsOver,
    SizeLimitExceeded,
    Common(CommonError),
}

// toolchains
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolchainInformation {
    pub name: String,
    pub id: u32,
}

/// This traits serve for documentation-only purposes
pub trait Frontend {
    /// POST /auth/anonymous
    fn auth_anonymous() -> Result<AuthToken, CommonError>;
    /// POST /auth/simple/
    fn auth_simple(auth_params: SimpleAuthParams) -> Result<AuthToken, SimpleAuthError>;

    /// POST /submissions/send
    fn submissions_send(sd: SubmitDeclaration) -> Result<SubmissionId, SubmitError>;

    /// GET /toolchains/list
    fn toolchains_list() -> Result<Vec<ToolchainInformation>, CommonError>;
}
