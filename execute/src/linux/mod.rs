mod dominion;
mod jail_common;
mod jobserver;
mod pipe;
mod util;

pub use crate::linux::dominion::LinuxDominion;
use crate::{
    linux::{
        pipe::{setup_pipe, LinuxReadPipe, LinuxWritePipe},
        util::{err_exit, ExitCode, Handle, Pid, Sock},
    },
    Backend, ChildProcess, ChildProcessOptions, ChildProcessStdio, DominionOptions,
    DominionPointerOwner, DominionRef, WaitResult,
};
use libc::c_void;
use std::{
    ffi::CString,
    mem, ptr,
    sync::{Arc, Mutex},
    time,
};

pub struct LinuxChildProcess {
    exit_code: Option<i64>,

    stdin: Handle,
    stdout: Handle,
    stderr: Handle,
    stdio_wasted: bool,
    //in order to save dominion while CP is alive
    _dominion_ref: DominionRef,

    pid: Pid,
}

struct WaiterArg {
    res_fd: Handle,
    pid: Pid,
}

extern "C" fn timed_wait_waiter(arg: *mut c_void) -> ExitCode {
    unsafe {
        let arg = arg as *mut WaiterArg;
        let arg = &mut *arg;
        let mut waitstatus = 0;
        let wcode = libc::waitpid(arg.pid, &mut waitstatus, libc::__WALL);
        if wcode == -1 {
            err_exit("waitpid");
        }
        let exit_code = if libc::WIFEXITED(waitstatus) {
            libc::WEXITSTATUS(waitstatus)
        } else {
            -libc::WTERMSIG(waitstatus)
        };
        let message = format!("{}", exit_code);
        let message_len = message.len();
        let message = CString::new(message).unwrap();
        libc::write(arg.res_fd, message.as_ptr() as *const _, message_len);
        0
    }
}

fn timed_wait(pid: Pid, timeout: time::Duration) -> Option<ExitCode> {
    unsafe {
        let (mut end_r, mut end_w);
        end_r = 0;
        end_w = 0;
        setup_pipe(&mut end_r, &mut end_w);
        let waiter_pid;
        {
            let waiter_stack = util::allocate_memory(1_048_576 + 4096); //1 MB
                                                                        //TODO fix leaks
            let waiter_stack = waiter_stack.add(4096 - (waiter_stack as usize % 4096));
            let arg = WaiterArg { res_fd: end_w, pid };
            let argp = util::allocate_heap_variable();
            *argp = arg;
            let argp = argp as *mut c_void;
            let cres = libc::clone(
                timed_wait_waiter,
                waiter_stack.add(1_048_576) as *mut c_void,
                libc::CLONE_THREAD | libc::CLONE_SIGHAND | libc::CLONE_VM,
                argp,
            );
            if cres == -1 {
                err_exit("clone");
            }

            waiter_pid = cres;
            libc::close(end_w);
        }
        //general idea - select([ready_r], timeout)
        let mut poll_fd_info: [libc::pollfd; 1];
        poll_fd_info = mem::zeroed();
        let mut poll_fd_ref = &mut poll_fd_info[0];
        poll_fd_ref.fd = end_r;
        poll_fd_ref.events = libc::POLLIN;
        let timeout = (timeout.as_secs() * 1000) as u32 + timeout.subsec_millis();
        let poll_ret = libc::poll(poll_fd_info.as_mut_ptr(), 1, timeout as i32);
        let ret = match poll_ret {
            -1 => err_exit("poll"),
            0 => None,
            1 => {
                let mut exit_code = [0; 16];
                let read_cnt = libc::read(end_r, exit_code.as_mut_ptr() as *mut c_void, 16);
                if read_cnt == -1 {
                    err_exit("read");
                }
                let exit_code = String::from_utf8(exit_code[..read_cnt as usize].to_vec()).unwrap();
                Some(exit_code.parse().unwrap())
            }
            x => unreachable!("unexpected return code from poll: {}", x),
        };
        libc::kill(waiter_pid, libc::SIGKILL);
        ret
    }
}

impl ChildProcess for LinuxChildProcess {
    type In = LinuxReadPipe;

    type Out = LinuxWritePipe;

    fn get_exit_code(&self) -> Option<i64> {
        self.exit_code
    }

    fn get_stdio(&mut self) -> Option<ChildProcessStdio<LinuxReadPipe, LinuxWritePipe>> {
        if self.stdio_wasted {
            None
        } else {
            self.stdio_wasted = true;
            Some(ChildProcessStdio {
                stdin: LinuxWritePipe::new(self.stdin),
                stdout: LinuxReadPipe::new(self.stdout),
                stderr: LinuxReadPipe::new(self.stderr),
            })
        }
    }

    fn wait_for_exit(&mut self, timeout: std::time::Duration) -> crate::Result<WaitResult> {
        if self.is_finished() {
            return Ok(WaitResult::AlreadyFinished);
        }
        let wait_result = timed_wait(self.pid, timeout);
        match wait_result {
            None => Ok(WaitResult::Timeout),
            Some(w) => {
                self.exit_code = Some(w as i64);
                Ok(WaitResult::Exited)
            }
        }
    }

    fn is_finished(&self) -> bool {
        self.exit_code.is_some()
    }

    fn kill(&mut self) {
        unsafe {
            if self.is_finished() {
                return;
            }
            if libc::kill(self.pid, libc::SIGKILL) == -1 {
                err_exit("kill");
            }
        }
    }
}

impl Drop for LinuxChildProcess {
    fn drop(&mut self) {
        if self.is_finished() {
            return;
        }
        self.kill();
        self.wait_for_exit(time::Duration::from_millis(100))
            .unwrap();
    }
}

fn spawn(options: ChildProcessOptions) -> LinuxChildProcess {
    unimplemented!()
}

pub struct LinuxBackend {}

impl Backend for LinuxBackend {
    type ChildProcess = LinuxChildProcess;
    fn new_dominion(&mut self, options: DominionOptions) -> DominionRef {
        let pd = unsafe { LinuxDominion::create(options) };
        DominionRef {
            d: Arc::new(Mutex::new(DominionPointerOwner { ptr: pd })),
        }
    }

    fn spawn(&mut self, options: ChildProcessOptions) -> LinuxChildProcess {
        spawn(options)
    }
}

fn empty_signal_handler(
    _signal_code: libc::c_int,
    _signal_info: *mut libc::siginfo_t,
    _ptr: *mut libc::c_void,
) {
}

fn fix_sigchild() {
    unsafe {
        let sa_ptr: *mut libc::sigaction = util::allocate_heap_variable();
        let mut sa = &mut *sa_ptr;
        sa.sa_sigaction = empty_signal_handler as *mut () as usize;
        libc::sigemptyset(&mut sa.sa_mask as *mut _);
        libc::sigaddset(&mut sa.sa_mask as *mut _, libc::SIGCHLD);
        sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
        if libc::sigaction(libc::SIGCHLD, sa_ptr, ptr::null_mut()) == -1 {
            err_exit("sigaction");
        }
    }
}

pub fn setup_execution_manager() -> LinuxBackend {
    fix_sigchild();
    LinuxBackend {}
}
