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

pub enum WaitResult {
    Exited,
    AlreadyFinished,
    Timeout
}

pub trait ChildProcess {
    fn get_exit_code(&self) -> Option<i64>;

    fn get_stdin(&mut self) -> &mut Write;

    fn get_stdout(&mut self) -> &mut Read;

    fn get_stderr(&mut self) -> &mut Read;

    fn wait_for_exit(&mut self, timeout: Duration) -> Result<WaitResult, std::io::Error>;

    fn is_finished(&self) -> bool;
}