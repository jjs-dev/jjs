mod linux;
mod definitions;

use definitions::*;
pub use definitions::{Result, Error, ChildProcessOptions, WaitResult};
//use std::{time::Duration, io::{Read, Write}};

/*pub struct ChildProcess {
    i: Box<dyn ChildProcessImpl>,
}*/

#[cfg(target_os = "linux")]
pub fn spawn(options: ChildProcessOptions) -> Result<Box<dyn ChildProcess>> {
    linux::spawn(options)
        //.map(|cpi| {
        //    ChildProcess {
        //        i: cpi,
        //    }
        //})
}

#[cfg(target_os = "linux")]
pub fn setup() -> Result<()> {
    linux::setup()
}

/*
impl ChildProcess {
    pub fn get_exit_code(&self) -> Option<i64> {
        self.i.get_exit_code()
    }

    pub fn get_stdin(&mut self) -> &mut Write {
        self.i.get_stdin()
    }

    pub fn get_stdout(&mut self) -> &mut Read {
        self.i.get_stdout()
    }

    pub fn get_stderr(&mut self) -> &mut Read {
        self.i.get_stderr()
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<WaitResult> {
        self.i.wait_for_exit(timeout)
    }

    pub fn is_finished(&self) -> bool {
        self.i.is_finished()
    }
}
*/
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
