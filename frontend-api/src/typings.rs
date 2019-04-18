#[macro_use]
extern crate serde_derive;

/// Represents errors, which can happen in (almost) each method.
#[derive(Debug, Serialize, Deserialize)]
pub enum CommonError {
    /// Authorization failed
    AccessDenied,
    /// Internal error in JJS, config, plugin, etc
    InternalError,
    /// Auth token is malformed or expired
    AuthTokenFault,
}

// some typedefs
pub type ToolchainId = u32;
pub type SubmissionId = u32;
pub type EmptyParams = ();

// auth
/// Opaque struct that represents auth token
/// You mustn't make any assumptions regarding 'buf' field, except that is ASCII string
/// without any whitespaces
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub buf: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthSimpleParams {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AuthSimpleError {
    UnknownLogin,
    IncorrectPassword,
    NotSuitable,
    Common(CommonError),
}

// submissions
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmissionSendParams {
    pub toolchain: ToolchainId,
    /// Must be correct base64-encoded string
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmitError {
    UnknownToolchain,
    ContestIsOver,
    SizeLimitExceeded,
    Base64,
    Common(CommonError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmissionInformation {
    pub id: SubmissionId,
    pub toolchain_name: String,
    pub status: String,
    pub score: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmissionListParams {
    pub limit: u32,
}

// toolchains
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolchainInformation {
    pub name: String,
    pub id: u32,
}

/// This traits serve for documentation-only purposes
///
/// Argument must be JSON-encoded and sent as a body (not form!)
pub trait Frontend {
    fn auth_anonymous(nope: EmptyParams) -> Result<AuthToken, CommonError>;

    fn auth_simple(auth_params: AuthSimpleParams) -> Result<AuthToken, AuthSimpleError>;

    fn submissions_send(sd: SubmissionSendParams) -> Result<SubmissionId, SubmitError>;

    fn submissions_list(selection_params: SubmissionListParams) -> Result<Vec<SubmissionInformation>, CommonError>;

    fn toolchains_list(nope: EmptyParams) -> Result<Vec<ToolchainInformation>, CommonError>;
}
