extern crate libc;
extern crate std;
extern crate errno;

use self::libc::{c_int, c_char, c_void};
use ::definitions::*;
use std::ffi::CString;
use std::io;

type H = c_int;
type Pid = libc::pid_t;

fn err_exit(func_name: &str, syscall_name: &str) {
    let e = errno::errno();
    panic!("{}: {}() failed with error {}: {}", func_name, syscall_name, e.0, e);
}


struct LinuxReadPipe {
    handle: H,
}

impl std::io::Read for LinuxReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        unsafe {
            let ret = libc::read(self.handle, buf.as_mut_ptr() as *mut c_void, buf.len());
            if ret == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(ret as usize)
        }
    }
}

struct LinuxWritePipe {
    handle: H,
}

impl io::Write for LinuxWritePipe {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        unsafe {
            let ret = libc::write(self.handle, buf.as_ptr() as *const c_void, buf.len());
            if ret == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(ret as usize)
        }
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        unsafe {
            let ret = libc::fsync(self.handle);
            if ret == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }
}

struct LinuxChildProcess {
    //handle: libc::c_int,
    exit_code: Option<i64>,
    stdin: LinuxWritePipe,
    stdout: LinuxReadPipe,
    stderr: LinuxReadPipe,
    pid: Pid,
    has_finished: bool,
}

fn timed_wait(pid: Pid, timeout: std::time::Duration) -> Option<i32> {
    unsafe {
        let timeout_millis = (timeout.as_secs() as i32).checked_mul(1000i32).unwrap()
            .checked_add((timeout.subsec_nanos() as i32).checked_div(1000i32).unwrap())
            .unwrap();
        /*let mut done_r: H = 0;
        let mut done_w: H = 0;
        setup_pipe(&mut done_r, &mut done_w).unwrap();
        let worker_pid = libc::fork();
        if worker_pid == -1 {
            err_exit("timed_wait", "fork");
        }
        if worker_pid == 0 {
            libc::close(done_r);
            //we are in child
            let mut waitstatus = 0;
            let wret;
            wret = libc::waitpid(pid, &mut waitstatus as *mut c_int, 0);
            if wret == -1 {
                err_exit("timed_wait", "waitpid");
            }
            let buf = waitstatus.to_string();
            //eprintln!("child: sending")
            let buf = buf.as_bytes();
            if libc::write(done_w, buf.as_ptr() as *const c_void, buf.len()) != (buf.len() as isize) {
                err_exit("timed_wait", "write");
            }
            libc::exit(0);
        } else {
            libc::close(done_w);
            let mut pfd = libc::pollfd {
                fd: done_r,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = libc::poll(&mut pfd as *mut libc::pollfd, 1, timeout_millis);
            if ret == -1 {
                err_exit("timed_wait", "poll");
            }
            if ret == 0 {
                if libc::kill(worker_pid, libc::SIGKILL) == -1 {
                    err_exit("timed_wait", "kill");
                }
                return None;
            }
            const BUFFER_SIZE: usize = 20;
            let buf =libc::malloc(BUFFER_SIZE);
            let read_res = libc::read(done_r, buf, BUFFER_SIZE);
            if read_res == -1 {
                err_exit("timed_wait", "read");
            }
            let buf = std::ffi::CString::from_raw(buf as *mut c_char);
            let buf = buf.to_str().unwrap();
            eprintln!("got from child: `{}` ({} bytes)", buf, read_res);
            let wstatus: i32 = buf.parse().unwrap();
            Some(wstatus)
            //todo wait for worker_pid
        }*/
    }
    None
}

impl ChildProcess for LinuxChildProcess {
    fn get_exit_code(&self) -> Option<i64> {
        self.exit_code
    }

    fn get_stdin(&mut self) -> &mut io::Write {
        &mut self.stdin
    }

    fn get_stdout(&mut self) -> &mut io::Read {
        &mut self.stdout
    }

    fn get_stderr(&mut self) -> &mut io::Read {
        &mut self.stderr
    }

    fn wait_for_exit(&mut self, timeout: std::time::Duration) -> Result<WaitResult, io::Error> {
        unsafe {
            if self.is_finished() {
                return Ok(WaitResult::AlreadyFinished);
            }
            let wait_result = timed_wait(self.pid, timeout);
            match wait_result {
                None => {
                     Ok(WaitResult::Timeout)
                }
                Some(w) => {
                    if libc::WIFEXITED(w) {
                        self.exit_code = Some(i64::from(libc::WEXITSTATUS(w)));
                    } else {
                        self.exit_code = Some(i64::from(-libc::WTERMSIG(w)));
                    }
                     Ok(WaitResult::Exited)
                }
            }
        }
    }

    fn is_finished(&self) -> bool {
        return self.has_finished;
    }
}

const POINTER_SIZE: usize = std::mem::size_of::<libc::c_void>();

struct DoExecArg {
    //in
    path: String,
    arguments: Vec<String>,
    environment: std::collections::HashMap<String, String>,
    stdin: H,
    stdout: H,
    stderr: H,
    //out
}

extern "C" fn do_exec(arg: *mut c_void) -> i32 {
    unsafe {
        let arg = &*(arg as *mut DoExecArg);
        let zpath = CString::new(arg.path.clone()).expect("path to executable contains zero byte");
        let path = libc::strdup(zpath.as_ptr());

        let mut argv_with_path = vec![arg.path.clone()];
        argv_with_path.append(&mut (arg.arguments.clone()));

        let argv = libc::malloc((argv_with_path.len() + 1) * POINTER_SIZE) as *const *const c_char;
        for (i, argument) in argv_with_path.iter().enumerate() {
            let zarg = CString::new(argument.clone()).unwrap_or_else(|_| panic!("argument #{} contains zero byte", i));

            let mut arg_copy = libc::strdup(zarg.as_ptr());
            *(argv.offset(i as isize) as *mut *const c_char) = arg_copy;
        }
        let envp = libc::malloc((arg.environment.len() + 1) * POINTER_SIZE) as *const *const c_char;
        for (i, (name, value)) in arg.environment.iter().enumerate() {
            let mut envp_item = format!("{}={}\0", name, value);
            let zitem = CString::new(envp_item).unwrap();
            let mut item_copy = libc::strdup(zitem.as_ptr());
            *(envp.offset(i as isize) as *mut *const c_char) = item_copy;
        }
        libc::dup2(arg.stdin, libc::STDIN_FILENO);
        libc::dup2(arg.stdout, libc::STDOUT_FILENO);
        libc::dup2(arg.stderr, libc::STDERR_FILENO);

        //now we need mark all FDs as CLOEXEC for not to expose them to judgee
        let fd_list_path = format!("/dev/{}/fd", std::process::id());
        eprintln!("fd_list_path={}", fd_list_path);
        let fd_list = std::fs::read_to_string(fd_list_path).unwrap();
        for fd in fd_list.split(" ") {
            let fd: H = fd.parse().unwrap();
            if -1 == libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) {
                panic!("couldn't cloexec fd: {}", fd);
            }
        }
        let ret = libc::execve(path, argv as *const *const c_char, envp as *const *const c_char);
        if ret == -1 {
            panic!("c error: {}", io::Error::last_os_error());
        }
    }
    0
}

fn setup_pipe(read_end: &mut H, write_end: &mut H) -> Result<(), io::Error> {
    unsafe {
        let mut sl = [0 as H; 2];
        let ret = libc::pipe(sl.as_mut_ptr());
        if ret == -1 {
            return Err(io::Error::last_os_error());
        }
        *read_end = sl[0];
        *write_end = sl[1];
        Ok(())
    }
}

const CHILD_STACK_SIZE: usize = 1024 * 1024;

pub fn spawn(options: ChildProcessOptions) -> Result<Box<dyn ChildProcess>, Error> {
    unsafe {
        let mut in_r = 0;
        let mut in_w = 0;
        let mut out_r = 0;
        let mut out_w = 0;
        let mut err_r = 0;
        let mut err_w = 0;

        setup_pipe(&mut in_r, &mut in_w).unwrap();
        setup_pipe(&mut out_r, &mut out_w).unwrap();
        setup_pipe(&mut err_r, &mut err_w).unwrap();
        let mut dea = DoExecArg {
            path: options.path,
            arguments: options.arguments,
            environment: options.environment,
            stdin: in_r,
            stdout: out_w,
            stderr: err_w,
        };
        let child_stack = libc::malloc(CHILD_STACK_SIZE);
        let dea_ptr = &mut dea as *mut DoExecArg;
        let child_pid = libc::clone(do_exec, ((child_stack as usize) + CHILD_STACK_SIZE) as
            *mut c_void, 0, dea_ptr as *mut c_void);

        Ok(Box::new(LinuxChildProcess {
            //handle: 0,
            exit_code: None,
            stdin: LinuxWritePipe {
                handle: in_w,
            },
            stdout: LinuxReadPipe {
                handle: out_r,
            },
            stderr: LinuxReadPipe {
                handle: err_r,
            },
            pid: child_pid,
            has_finished: false,
        }))
    }
}