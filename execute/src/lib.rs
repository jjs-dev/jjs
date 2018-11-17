#[cfg(target_os = "linux")]
mod linux;

use cfg_if::cfg_if;
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Read, Write},
    mem,
    //sync::{Mutex, Arc},
    time::Duration,
};

pub trait ExecutionManager {
    type ChildProcess: ChildProcess;
    fn new_dominion(&mut self, options: DominionOptions) -> DominionRef;
    fn spawn(&mut self, options: ChildProcessOptions) -> Self::ChildProcess;
}

pub struct DominionOptions {
    pub allow_network: bool,
    pub allow_file_io: bool,
    pub max_alive_process_count: usize,
    pub memory_limit: usize,
}

///RAII object which represents highly-isolated sandbox
pub trait Dominion: Debug {}

#[cfg(target_os = "linux")]
pub type SelectedDominion = linux::LinuxDominion;

#[derive(Clone, Debug)]
pub struct DominionRef {
    d: *mut SelectedDominion,
}

cfg_if! {
    if #[cfg(target_os = "linux")] {
        fn drop_dom_ref(dref: &mut DominionRef) {
            let inner = dref.d as *mut SelectedDominion;
            unsafe {
                mem::drop(&mut *inner);
            }
        }
    }
}

impl Drop for DominionRef {
    fn drop(&mut self) {
        drop_dom_ref(self);
    }
}

//pub type Dominion = linux::LinuxDominion;

#[derive(Debug)]
pub struct ChildProcessOptions {
    pub path: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub dominion: DominionRef,
}

#[derive(Debug)]
pub enum Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub enum WaitResult {
    Exited,
    AlreadyFinished,
    Timeout,
}

pub struct ChildProcessStdio<In: Read, Out: Write> {
    pub stdin: Out,
    pub stdout: In,
    pub stderr: In,
}

impl<In: Read, Out: Write> ChildProcessStdio<In, Out> {
    pub fn split(self) -> (Out, In, In) {
        (self.stdin, self.stdout, self.stderr)
    }
}

///represents child process, owns it.
pub trait ChildProcess: Drop {
    type In: Read;

    type Out: Write;

    ///returns exit code, it process had exited by the moment of call, or None otherwise.
    fn get_exit_code(&self) -> Option<i64>;

    fn get_stdio(&mut self) -> Option<ChildProcessStdio<Self::In, Self::Out>>;

    ///waits for child process exit with timeout
    fn wait_for_exit(&mut self, timeout: Duration) -> Result<WaitResult>;

    ///returns whether child process has exited
    fn is_finished(&self) -> bool;

    ///Kills underlying process ASAP (e.g. kill() or TerminateProcess())
    fn kill(&mut self);
}

#[cfg(target_os = "linux")]
pub type MinionExecutionManager = linux::LinuxEM;

#[cfg(target_os = "linux")]
pub fn setup() -> MinionExecutionManager {
    linux::setup_execution_manager()
}
