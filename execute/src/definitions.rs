extern crate std;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::Duration;

pub struct ChildProcessOptions {
    pub path: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
}

#[derive(Debug)]
pub enum Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub enum WaitResult {
    Exited,
    AlreadyFinished,
    Timeout,
}

///represents child process, owns it.
pub trait ChildProcess: Drop {
    ///returns exit code, it process had exited by the moment of call, or None otherwise.
    fn get_exit_code(&self) -> Option<i64>;

    ///returns Writable stream for stdin
    fn get_stdin(&mut self) -> &mut Write;

    ///returns Readable stream for stdout
    fn get_stdout(&mut self) -> &mut Read;

    ///returns Readable stream for stderr
    fn get_stderr(&mut self) -> &mut Read;

    ///waits for child process exit with timeout
    fn wait_for_exit(&mut self, timeout: Duration) -> Result<WaitResult>;

    ///returns whether child process has exited
    fn is_finished(&self) -> bool;

    ///Kills underlying process ASAP (e.g. kill() or TerminateProcess())
    fn kill(&mut self);
}