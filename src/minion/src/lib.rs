/*!
 * This crate provides ability to spawn highly isolated processes
 *
 * # Platform support
 * _warning_: not all features are supported by all backends. See documentation for particular backend
 * to know more
 */
mod command;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::check::check as linux_check_environment;

use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
pub use crate::linux::{LinuxBackend, LinuxChildProcess, LinuxDominion};

use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Read, Write},
    sync::Arc,
    time::Duration,
};

/// Represents way of isolation
pub trait Backend: Debug + Send + Sync {
    fn new_dominion(&self, options: DominionOptions) -> Result<DominionRef>;
    fn spawn(&self, options: ChildProcessOptions) -> Result<Box<dyn ChildProcess>>;
}

#[cfg(target_os = "linux")]
pub use {linux::DesiredAccess, linux::LinuxHandle};

pub use command::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PathExpositionOptions {
    /// Path on system
    pub src: PathBuf,
    /// Path for child
    pub dest: PathBuf,
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
    /// Specifies total CPU time for whole dominion
    pub cpu_time_limit: Duration,
    /// Specifies total wall-clock timer limit for whole dominion
    pub real_time_limit: Duration,
    pub isolation_root: PathBuf,
    pub exposed_paths: Vec<PathExpositionOptions>,
}

impl DominionOptions {
    fn make_relative<'a>(&self, p: &'a Path) -> &'a Path {
        if p.starts_with("/") {
            p.strip_prefix("/").unwrap()
        } else {
            p
        }
    }

    fn postprocess(&mut self) {
        let mut paths = std::mem::replace(&mut self.exposed_paths, Vec::new());
        for x in &mut paths {
            x.dest = self.make_relative(&x.dest).to_path_buf();
        }
        std::mem::swap(&mut paths, &mut self.exposed_paths);
    }
}

/// Represents highly-isolated sandbox
pub trait Dominion: Debug + std::any::Any + 'static {
    fn as_any(&self) -> &(dyn std::any::Any + 'static)
    where
        Self: Sized,
    {
        self
    }

    fn id(&self) -> String;

    /// Returns true if dominion exceeded CPU time limit
    fn check_cpu_tle(&self) -> Result<bool>;

    /// Returns true if dominion exceeded wall-clock time limit
    fn check_real_tle(&self) -> Result<bool>;
}

#[derive(Debug)]
enum DominionRefInner {
    Linux(LinuxDominion),
}

/// Type-erased dominion
#[derive(Clone, Debug)]
pub struct DominionRef(Arc<DominionRefInner>);

impl DominionRef {
    // Private downcasting support

    /// Downcast to LinuxDominion.
    /// If `self` contains some other, this function will panic.
    pub(crate) fn downcast_linux(&self) -> &LinuxDominion {
        match &*self.0 {
            DominionRefInner::Linux(lx) => lx,
        }
    }
}

impl From<LinuxDominion> for DominionRef {
    fn from(lx: LinuxDominion) -> Self {
        DominionRef(Arc::new(DominionRefInner::Linux(lx)))
    }
}

impl std::ops::Deref for DominionRefInner {
    type Target = dyn Dominion;

    fn deref(&self) -> &dyn Dominion {
        match self {
            DominionRefInner::Linux(lx) => lx,
        }
    }
}

impl Dominion for DominionRef {
    fn id(&self) -> String {
        self.0.id()
    }

    fn check_cpu_tle(&self) -> Result<bool> {
        self.0.check_cpu_tle()
    }

    fn check_real_tle(&self) -> Result<bool> {
        self.0.check_real_tle()
    }
}

/// Configures stdin for child
#[derive(Debug, Clone)]
enum InputSpecificationData {
    Null,
    Empty,
    Pipe,
    Handle(u64),
}

#[derive(Debug, Clone)]
pub struct InputSpecification(InputSpecificationData);

impl InputSpecification {
    pub fn null() -> Self {
        Self(InputSpecificationData::Null)
    }

    pub fn empty() -> Self {
        Self(InputSpecificationData::Empty)
    }

    pub fn pipe() -> Self {
        Self(InputSpecificationData::Pipe)
    }

    /// # Safety
    /// - Handle must not be used since passing to this function
    /// - Handle must be valid
    pub unsafe fn handle(h: u64) -> Self {
        Self(InputSpecificationData::Handle(h))
    }

    /// # Safety
    /// See requirements of `handle`
    pub unsafe fn handle_of<T: std::os::unix::io::IntoRawFd>(obj: T) -> Self {
        Self::handle(obj.into_raw_fd() as u64)
    }
}

/// Configures stdout and stderr for child
#[derive(Debug, Clone)]
enum OutputSpecificationData {
    Null,
    Ignore,
    Pipe,
    Buffer(Option<usize>),
    Handle(u64),
}

impl OutputSpecification {
    pub fn null() -> Self {
        Self(OutputSpecificationData::Null)
    }

    pub fn ignore() -> Self {
        Self(OutputSpecificationData::Ignore)
    }

    pub fn pipe() -> Self {
        Self(OutputSpecificationData::Pipe)
    }

    pub fn buffer(size: usize) -> Self {
        Self(OutputSpecificationData::Buffer(Some(size)))
    }

    pub fn unbounded_buffer() -> Self {
        Self(OutputSpecificationData::Buffer(None))
    }

    /// # Safety
    /// - Handle must not be used since passing to this function
    /// - Handle must be valid
    pub unsafe fn handle(h: u64) -> Self {
        Self(OutputSpecificationData::Handle(h))
    }

    /// # Safety
    /// See requirements of `handle`
    pub unsafe fn handle_of<T: std::os::unix::io::IntoRawFd>(obj: T) -> Self {
        Self::handle(obj.into_raw_fd() as u64)
    }
}

#[derive(Debug, Clone)]
pub struct OutputSpecification(OutputSpecificationData);

/// Specifies how to provide child stdio
#[derive(Debug, Clone)]
pub struct StdioSpecification {
    pub stdin: InputSpecification,
    pub stdout: OutputSpecification,
    pub stderr: OutputSpecification,
}

/// This type should only be used by Backend implementations
/// Use `Command` instead
#[derive(Debug, Clone)]
pub struct ChildProcessOptions {
    pub path: PathBuf,
    pub arguments: Vec<OsString>,
    pub environment: HashMap<OsString, OsString>,
    pub dominion: DominionRef,
    pub stdio: StdioSpecification,
    /// Child's working dir. Relative to `dominion` isolation_root
    pub pwd: PathBuf,
}

mod errors {
    use snafu::Snafu;

    #[derive(Eq, PartialEq)]
    pub enum ErrorKind {
        /// This error typically means that isolated process tried to break its sandbox
        Sandbox,
        /// Bug in code, using minion, or in minion itself
        System,
    }

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    pub enum Error {
        #[snafu(display("requested operation is not supported by backend"))]
        NotSupported,
        #[snafu(display("system call failed in undesired fashion (error code {})", code))]
        System { code: i32 },
        #[snafu(display("io error"))]
        Io { source: std::io::Error },
        #[snafu(display("sandbox interaction failed"))]
        Sandbox,
        #[snafu(display("unknown error"))]
        Unknown,
    }

    impl Error {
        pub fn kind(&self) -> ErrorKind {
            match self {
                Error::NotSupported => ErrorKind::System,
                Error::System { .. } => ErrorKind::System,
                Error::Io { .. } => ErrorKind::System,
                Error::Sandbox => ErrorKind::Sandbox,
                Error::Unknown => ErrorKind::System,
            }
        }

        pub fn is_system(&self) -> bool {
            self.kind() == ErrorKind::System
        }

        pub fn is_sandbox(&self) -> bool {
            self.kind() == ErrorKind::Sandbox
        }
    }

    impl From<nix::Error> for Error {
        fn from(err: nix::Error) -> Self {
            if let Some(errno) = err.as_errno() {
                Error::System { code: errno as i32 }
            } else {
                Error::Unknown
            }
        }
    }
}

pub use errors::Error;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

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

/// Represents child process.
pub trait ChildProcess {
    /// Returns exit code, if process had exited by the moment of call, or None otherwise.
    fn get_exit_code(&self) -> Result<Option<i64>>;

    /// Returns writeable stream, connected to child stdin
    ///
    /// Stream will only be returned, if corresponding `Stdio` item was `new_pipe`.
    /// Otherwise, None will be returned
    ///
    /// On all subsequent calls, None will be returned

    fn stdin(&mut self) -> Option<Box<dyn Write + Send + Sync>>;

    /// Returns readable stream, connected to child stdoutn
    ///
    /// Stream will only be returned, if corresponding `Stdio` item was `new_pipe`.
    /// Otherwise, None will be returned
    ///
    /// On all subsequent calls, None will be returned
    fn stdout(&mut self) -> Option<Box<dyn Read + Send + Sync>>;

    /// Returns readable stream, connected to child stderr
    ///
    /// Stream will only be returned, if corresponding `Stdio` item was `new_pipe`.
    /// Otherwise, None will be returned
    ///
    /// On all subsequent calls, None will be returned
    fn stderr(&mut self) -> Option<Box<dyn Read + Send + Sync>>;

    /// Waits for child process exit with timeout
    fn wait_for_exit(&self, timeout: Duration) -> Result<WaitOutcome>;

    /// Refreshes information about process
    fn poll(&self) -> Result<()>;

    /// Returns whether child process has exited by the moment of call
    /// This function doesn't blocks on waiting (see `wait_for_exit`).
    fn is_finished(&self) -> Result<bool>;
}

#[cfg(target_os = "linux")]
pub type DefaultBackend = linux::LinuxBackend;

#[cfg(target_os = "linux")]
pub fn setup() -> Box<dyn Backend> {
    Box::new(linux::setup_execution_manager())
}
