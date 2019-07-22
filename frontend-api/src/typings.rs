#[macro_use]
extern crate serde_derive;

/// Represents errors, which can happen in (almost) each method.
pub enum CommonError {
    /// Authorization failed
    AccessDenied,
    /// Internal error in JJS, config, plugin, etc
    InternalError,
    /// Auth token is malformed or expired
    AuthTokenFault,
    /// Resource specified was not found
    NotFound,
    /// Resource was deleted
    Gone,
}

// some typedefs
pub type ToolchainId = u32;
pub type SubmissionId = u32;
pub type ProblemCode = String;
pub type ContestId = String;
pub type EmptyParams = ();

// auth
/// Opaque struct that represents auth token
/// You mustn't make any assumptions regarding 'buf' field, except that is ASCII string
/// without any whitespaces
pub struct AuthToken {
    pub buf: String,
}

pub struct AuthSimpleParams {
    pub login: String,
    pub password: String,
}

pub enum AuthSimpleError {
    UnknownLogin,
    IncorrectPassword,
    NotSuitable,
    Common(CommonError),
}

// submissions
pub struct SubmissionSendParams {
    pub toolchain: ToolchainId,
    /// Must be correct base64-encoded string
    pub code: String,
    pub problem: ProblemCode,
    pub contest: ContestId,
}

pub enum SubmitError {
    UnknownProblem,
    UnknownContest,
    UnknownToolchain,
    ContestIsOver,
    SizeLimitExceeded,
    Base64,
    Common(CommonError),
}

pub struct JudgeStatus {
    pub kind: String,
    pub code: String,
}

pub enum SubmissionState {
    Queue,
    Judge,
    Finish,
    Error,
}

pub struct SubmissionInformation {
    pub id: SubmissionId,
    pub toolchain_name: String,
    pub status: JudgeStatus,
    pub state: SubmissionState,
    pub score: Option<i32>,
    pub problem: ProblemCode,
}

pub struct SubmissionsListParams {
    pub limit: u32,
}

pub struct SubmissionsSetInfoParams {
    pub id: SubmissionId,
    pub status: Option<JudgeStatus>,
    pub state: Option<SubmissionState>,
    pub rejudge: bool,
    pub delete: bool,
}

pub enum SubmissionsBlobQuery {
    Source,
    BuildArtifact,
}

pub struct SubmissionsBlobParams {
    pub id: SubmissionId,
    pub query: SubmissionsBlobQuery,
}

pub enum Blob {
    Data(Vec<u8>),
    Url(String),
}

// toolchains
pub struct ToolchainInformation {
    pub name: String,
    pub id: ToolchainId,
}

// users
pub struct UsersCreateParams {
    pub login: String,
    pub password: String,
    pub groups: Vec<String>,
}

pub enum UsersCreateError {
    InvalidLogin,
    PasswordRejected,
    Common(CommonError),
}

// contests
pub struct ContestInformation {
    /// E.g. "Berlandian Olympiad in Informatics. Finals. Day 3."
    pub title: String,
    /// Configured by human, something readable like 'olymp-2019', or 'test-contest'
    pub name: ContestId,
    /// Only present in long form
    /// ProblemInformation itself is provided in short form
    pub problems: Option<Vec<ProblemInformation>>,
}

// problems
pub struct ProblemInformation {
    /// E.g. "Palindromic refrain"
    pub title: String,
    /// E.g. "F", "A1", "7"
    pub code: String,
}

/// This trait serves for documentation-only purposes
///
/// Argument must be JSON-encoded and sent as a body (not form!)
pub trait Frontend {
    fn auth_anonymous(nope: EmptyParams) -> Result<AuthToken, CommonError>;

    fn auth_simple(auth_params: AuthSimpleParams) -> Result<AuthToken, AuthSimpleError>;

    fn submissions_send(sd: SubmissionSendParams) -> Result<SubmissionId, SubmitError>;

    fn submissions_list(selection_params: SubmissionsListParams) -> Result<Vec<SubmissionInformation>, CommonError>;

    fn submissions_modify(info: SubmissionsSetInfoParams) -> Result<(), CommonError>;

    fn submissions_blob(params: SubmissionsBlobParams) -> Result<Vec<u8>, CommonError>;

    fn toolchains_list(nope: EmptyParams) -> Result<Vec<ToolchainInformation>, CommonError>;

    fn users_create(params: UsersCreateParams) -> Result<(), UsersCreateError>;

    /// Returns information about contests available
    /// Note that some contests can be missing due to their security policy
    fn contests_list(params: EmptyParams) -> Result<Vec<ContestInformation>, CommonError>;

    fn contests_describe(contest_id: ContestId) -> Result<ContestInformation, CommonError>;
}
