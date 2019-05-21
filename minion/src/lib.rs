/*!
* This crate provides ability to spawn highly isolated processes
*
* # Platform support
* _warning_: not all features are supported by all backends. See documentation for particular backend
* to know more
*/

#[macro_use]
extern crate serde_derive;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use crate::linux::{LinuxBackend, LinuxChildProcess, LinuxDominion};

use downcast_rs::impl_downcast;
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    io::{Read, Write},
    sync::{Arc, Mutex},
    time::Duration,
};
/// Represents way of isolation
pub trait Backend: Debug + Send + Sync {
    fn new_dominion(&self, options: DominionOptions) -> Result<DominionRef>;
    fn spawn(&self, options: ChildProcessOptions) -> Result<Box<dyn ChildProcess>>;
}

#[cfg(target_os = "linux")]
pub use crate::linux::DesiredAccess;
use failure::Fail;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PathExpositionOptions {
    pub src: String,
    pub dest: String,
    pub access: DesiredAccess,
}

/// This struct is returned by `Dominion::query_usage_data`
/// It represents various resource usage
/// Some items can be absent or rounded
pub struct ResourceUsageData {
    /// Total CPU time usage in nanoseconds
    pub time: Option<u64>,
    /// Max memory usage in bytes
    pub memory: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DominionOptions {
    pub max_alive_process_count: u32,
    /// Memory limit for all processes in cgroup, in bytes
    pub memory_limit: u64,
    /// Specifies total CPU time for all dominion
    pub time_limit: Duration,
    pub isolation_root: String,
    pub exposed_paths: Vec<PathExpositionOptions>,
}

/// Represents highly-isolated sandbox
pub trait Dominion: Debug + downcast_rs::Downcast {
    fn id(&self) -> String;
}
impl_downcast!(Dominion);

#[cfg(target_os = "linux")]
pub type SelectedDominion = LinuxDominion;

#[derive(Debug)]
struct DominionPointerOwner {
    b: Box<dyn Dominion>,
}

unsafe impl Send for DominionPointerOwner {}

#[derive(Clone, Debug)]
pub struct DominionRef {
    d: Arc<Mutex<DominionPointerOwner>>,
}

impl DominionRef {
    pub fn id(&self) -> String {
        self.d.lock().unwrap().b.id()
    }
}

#[derive(Debug, Clone)]
pub struct HandleWrapper {
    h: u64,
}

impl HandleWrapper {
    pub unsafe fn new(handle: u64) -> Self {
        Self { h: handle }
    }

    #[cfg(unix)]
    pub unsafe fn from<T: std::os::unix::io::IntoRawFd>(obj: T) -> Self {
        Self::new(obj.into_raw_fd() as u64)
    }
}

/// Configures stdin for child
#[derive(Debug, Clone)]
pub enum InputSpecification {
    Null,
    Empty,
    Pipe,
    RawHandle(HandleWrapper),
}

/// Configures stdout and stderr for child
#[derive(Debug, Clone)]
pub enum OutputSpecification {
    Null,
    Ignore,
    Pipe,
    Buffer(Option<usize>),
    RawHandle(HandleWrapper),
}

/// Specifies how to provide child stdio
#[derive(Debug, Clone)]
pub struct StdioSpecification {
    pub stdin: InputSpecification,
    pub stdout: OutputSpecification,
    pub stderr: OutputSpecification,
}

#[derive(Debug, Clone)]
pub struct ChildProcessOptions {
    pub path: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub dominion: DominionRef,
    pub stdio: StdioSpecification,
    /// Child's working dir. Relative to `dominion` isolation_root
    pub pwd: String,
}

#[derive(Debug)]
pub struct Error {
    inner: failure::Context<ErrorKind>,
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&failure::Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, fmt)
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(k: ErrorKind) -> Error {
        Error {
            inner: failure::Context::new(k),
        }
    }
}

impl From<failure::Context<ErrorKind>> for Error {
    fn from(c: failure::Context<ErrorKind>) -> Error {
        Error { inner: c }
    }
}

#[derive(Debug, Fail, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ErrorKind {
    #[fail(display = "requested operation is not supported by backend")]
    NotSupported,
    #[fail(
        display = "system call failed in undesired fashion (error code {})",
        _0
    )]
    System(i32),
    #[fail(display = "io error")]
    Io,
    #[fail(display = "job server connection failed")]
    Communication,
    #[fail(display = "unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, Error>;

/// Returned by [ChildProcess::wait_for_exit]
///
/// [ChildProcess::wait_fot_exit]: trait.ChildProcess.html#tymethod.wait_for_exit
pub enum WaitOutcome {
    /// Child process has exited during `wait_for_exit`
    Exited,
    /// Child process has exited before `wait_for_exit` and it is somehow already reported
    AlreadyFinished,
    /// Child process hasn't exited during `timeout` period
    Timeout,
}

pub type ChildInputStream = Box<dyn Write>;
pub type ChildOutputStream = Box<dyn Read>;
pub type ChildStdio = (
    Option<ChildInputStream>,
    Option<ChildOutputStream>,
    Option<ChildOutputStream>,
);

/// Represents child process.
pub trait ChildProcess: Drop {
    /// Returns exit code, if process had exited by the moment of call, or None otherwise.
    fn get_exit_code(&self) -> Result<Option<i64>>;

    /// Returns streams, connected to child stdio
    ///
    /// Stream will only be returned, if corresponding `Stdio` item was `new_pipe`.
    /// Otherwise, None will be returned
    ///
    /// On all subsequent calls, (None, None, None) will be returned - `stdio` transfers ownership

    fn stdio(&mut self) -> ChildStdio;

    /// Waits for child process exit with timeout
    fn wait_for_exit(&self, timeout: Duration) -> Result<WaitOutcome>;

    /// Refreshes information about process
    fn poll(&self) -> Result<()>;

    /// Returns whether child process has exited by the moment of call
    /// This function doesn't blocks on waiting (see `wait_for_exit`).
    fn is_finished(&self) -> Result<bool>;

    /// Kills underlying process as soon as possible
    fn kill(&mut self) -> Result<()>;
}

#[cfg(target_os = "linux")]
pub type DefaultBackend = linux::LinuxBackend;

#[cfg(target_os = "linux")]
pub fn setup() -> Box<dyn Backend> {
    Box::new(linux::setup_execution_manager())
}
