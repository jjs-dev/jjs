//! Defines Invoker API.
//! You can use invoker to securely executed untrusted programs.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct Invoke(std::convert::Infallible);

/// Requests invoker to execute commands, specified in
/// `steps` field in request.
/// # Execution order
/// Each step has assigned `stage`.
/// Steps with equal stage will be executed in the same time.
/// Such steps can share pipes. Sharing pipes between steps from
/// different stages results in error. For each stage,
/// steps creating new IPC stuff are executed first.
/// Step will not be executed until all steps with less `stage`
/// will be finished.
/// # Data
/// `InvokeRequest` can specify input data items, that can be further used
/// as stdin for executed commands (input data item can be used several times).
/// # DataRequest
/// `InvokeRequest` can specify output data requests, which will be populated
/// from some files, created by `CreateFile` action.
impl rpc::Route for Invoke {
    type Request = rpc::Unary<InvokeRequest>;
    type Response = rpc::Unary<InvokeResponse>;

    const ENDPOINT: &'static str = "/invoke";
}

#[derive(Serialize, Deserialize)]
pub struct InvokeRequest {
    /// Set of commands that must be executed
    pub steps: Vec<Step>,
    /// Binary data used for executing commands
    pub inputs: Vec<Input>,
    /// Binary data produced by executing commands
    pub outputs: Vec<OutputRequest>,
}

#[derive(Serialize, Deserialize)]
pub struct InvokeResponse {}

#[derive(Serialize, Deserialize)]
pub struct OutputRequest {
    /// File id that will later receive the data
    pub id: FileId,
}

#[derive(Serialize, Deserialize)]
pub struct Input {
    /// File id that must be assigned to this input
    pub id: FileId,
    /// Data source
    pub source: InputSource,
}

#[derive(Serialize, Deserialize)]
pub enum InputSource {
    /// Data available as file on FS
    LocalFile { path: PathBuf },
    /// Data provided inline
    Inline { data: Vec<u8> },
}

pub struct Output {}

#[derive(Serialize, Deserialize)]
pub struct Step {
    pub stage: u32,
    pub action: Action,
}

/// Newtype identifier of file-like object, e.g. real file or pipe.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileId(pub String);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Command {
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub cwd: String,
    pub stdio: Stdio,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stdio {
    pub stdin: FileId,
    pub stdout: FileId,
    pub stderr: FileId,
}

/// Single action of execution plan.
#[derive(Serialize, Deserialize)]
pub enum Action {
    /// Specifies that a pipe must be allocated
    CreatePipe {
        /// Will be associated with pipe's read half
        read: FileId,
        /// Will be associated with pipe's write half
        write: FileId,
    },
    /// Specifies that a file must be created
    CreateFile {
        /// Will be associated with the file
        id: FileId,
    },
    /// Associates file on local fs with a FileId
    OpenFile {
        /// Path to the file
        path: PathBuf,
        /// Id to associate with file
        id: FileId,
    },
    /// Associates file id with empty file, e.g. `/dev/null`
    OpenNullFile { id: FileId },
    /// Specifies that command should be executed
    ExecuteCommand(Command),
}
