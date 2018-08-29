extern crate libc;
extern crate std;
extern crate errno;
extern crate core;

use self::libc::{c_int, c_char, c_void};
use ::definitions::*;
use std::ffi::CString;
use std::{io, sync::{Mutex, Arc, Condvar}, thread, ptr};

type H = c_int;
type Pid = libc::pid_t;

fn err_exit(func_name: &str, syscall_name: &str) -> ! {
    let e = errno::errno();
    panic!("{}: {}() failed with error {}: {}", func_name, syscall_name, e.0, e);
}

pub fn setup() -> Result<()> {
    unsafe {
        //let mut act: libc::sigaction = std::mem::zeroed();
        //act.sa_handler = libc::SIG_IGN;
        //act.sa_flags = libc::SA_RESTART;
        //libc::sigaction(libc::SIGCHLD, ((&mut act).as_mut_ptr() as *mut libc::sigaction), ptr::null());
    }
    Ok(())
}

struct LinuxReadPipe {
    handle: H,
}

impl std::io::Read for LinuxReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            //println!("reading up to {} bytes from {}", buf.len(), self.handle);
            let ret = libc::read(self.handle, buf.as_mut_ptr() as *mut c_void, buf.len());
            if ret == -1 {
                err_exit("LinuxReadPipe::read", "read");
            }
            //println!("got {} bytes", ret);
            Ok(ret as usize)
        }
    }
}

struct LinuxWritePipe {
    handle: H,
}

impl io::Write for LinuxWritePipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::write(self.handle, buf.as_ptr() as *const c_void, buf.len());
            if ret == -1 {
                err_exit("LinuxWritePipe::write()", "write");
                //return Err(std::io::Error::last_os_error());
            }
            Ok(ret as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        unsafe {
            let ret = libc::fsync(self.handle);
            if ret == -1 {
                err_exit("LinuxWritePipe::flush", "fsync");
                //return Err(io::Error::last_os_error());
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

//unsafe impl Copy for Pid {}

macro_rules! SYNC {
($mutex_name:ident) => {

(*$mutex_name).lock().unwrap()

}
}

fn timed_wait(pid: Pid, timeout: std::time::Duration) -> Option<i32> {
    use std::{
        os::{
            unix::thread::JoinHandleExt
        }
    };
    struct Inter {
        //in
        pid: Pid,
        //out
        exited: bool,
        exit_code: i32,
    }
    unsafe {
        //let timeout_millis = (timeout.as_secs() as i32).checked_mul(1000i32).unwrap()
        //  .checked_add((timeout.subsec_nanos() as i32).checked_div(1000i32).unwrap())
        //.unwrap();
        let m = Arc::new(Mutex::new(Inter {
            pid,
            exited: false,
            exit_code: 0,
        }));
        //TODO rewrite
        let cv_should_return = Arc::new((Mutex::new(false), Condvar::new()));
        let mwaiter = m.clone();
        let cv_waiter = cv_should_return.clone();
        let waiter = std::thread::spawn(move || {
            let &(ref lock, ref cv) = &*cv_waiter;
            let m = mwaiter;
            let mut waitstatus = 0;
            let wcode = libc::waitpid(SYNC!(m).pid, &mut waitstatus, libc::__WALL);
            if wcode == -1 {
                err_exit("timed_wait", "waitpid");
            }
            //eprintln!("waiter: child process returned!!");
            {
                //let mut datag = m.lock().unwrap();
                SYNC!(m).exit_code = 228;
                SYNC!(m).exited = true;
                //eprintln!("set first mutex");
                *SYNC!(lock) = true;
                //eprintln!("set muticies");
                cv.notify_all();
                //eprintln!("notified condvar");
            }
        });
        let mtimeouter = m.clone();
        let cvtimeouter = cv_should_return.clone();
        let _timeouter = thread::spawn(move || {
            let m = mtimeouter.clone();
            let &(ref lock, ref cv) = &*cvtimeouter;
            thread::sleep(timeout.clone());
            if SYNC!(m).exited {
                return;
            }
            let waiter_handle = waiter.as_pthread_t();
            libc::pthread_cancel(waiter_handle);
            *SYNC!(lock) = true;
            cv.notify_all();
        });
        {
            let &(ref lock, ref cv) = &*cv_should_return;
            loop {
                let grd = lock.lock().unwrap();
                if *grd == true {
                    break;
                }
                cv.wait(grd).unwrap();
            }
        }
        //eprintln!("child is exited or killed.");
        return Some(SYNC!(m).exit_code);
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

    fn wait_for_exit(&mut self, timeout: std::time::Duration) -> Result<WaitResult> {
        unsafe {
            if self.is_finished() {
                return Ok(WaitResult::AlreadyFinished);
            }
            let wait_result = timed_wait(self.pid, timeout);
            //println!("timed_wait() returned {:?}", wait_result);
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

const POINTER_SIZE: usize = std::mem::size_of::<usize>();

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

fn duplicate_string(arg: &str) -> *mut c_char {
    unsafe {
        let cstr = CString::new(arg).unwrap();
        let strptr = cstr.as_ptr();
        let out = libc::strdup(strptr);
        out
    }
}

macro_rules! ptr_subscript_set {
    ($ptr: ident,  $ind: expr, $val: expr) => {
        *($ptr.offset(($ind) as isize)) = $val;
    }
}

fn allocate_memory(num: usize) -> *mut c_char {
    unsafe {
        let p = libc::malloc(num) as *mut c_char;
        ptr::write_bytes(p, 0xCD, num);
        return p;
    }
}

extern "C" fn do_exec(arg: *mut c_void) -> i32 {
    use std::iter::FromIterator;
    unsafe {
        let arg = &*(arg as *mut DoExecArg);
        //let zpath = CString::new(arg.path.clone()).expect("path to executable contains zero byte");
        //let path = libc::strdup(zpath.as_ptr());
        let path = duplicate_string(&arg.path);

        let mut argv_with_path = vec![arg.path.clone()];
        argv_with_path.append(&mut (arg.arguments.clone()));


        let num_argv_items = argv_with_path.len() + 1;
        let argv = allocate_memory(num_argv_items * POINTER_SIZE) as *mut *const c_char;
        //let argv = libc::malloc(num_argv_items * POINTER_SIZE) as *mut *const c_char;
        for (i, argument) in argv_with_path.iter().enumerate() {
            *(argv.offset(i as isize) as *mut *const c_char) = duplicate_string(argument);
        }
        ptr_subscript_set!(argv, num_argv_items-1, ptr::null());
        //ptr::write(argv.offset((num_argv_items - 1) as isize), ptr::null());
        //*(argv.offset(num_argv_items - 1)) = std::ptr::null();
        //println!("{} items in argv buffer", num_argv_items);
        for i in 0..num_argv_items {
            //println!("item #{} : address={}", i, *argv.offset(i as isize) as usize);
        }
        let num_envp_items = arg.environment.len() + 1;
        let envp = libc::malloc(num_envp_items * POINTER_SIZE) as *mut *const c_char;
        for (i, (name, value)) in arg.environment.iter().enumerate() {
            let mut envp_item = format!("{}={}", name, value);
            *(envp.offset(i as isize) as *mut *const c_char) = duplicate_string(&envp_item);
        }
        ptr_subscript_set!(envp, num_envp_items-1, ptr::null());
        //ptr::write(pt)

        //now we need mark all FDs as CLOEXEC for not to expose them to judgee
        //println!("My pid is {}", std::process::id());

        let fds_to_keep = vec![arg.stdin, arg.stdout, arg.stderr];
        let fds_to_keep = std::collections::BTreeSet::from_iter(fds_to_keep.iter());

        let fd_list_path = format!("/proc/{}/fd", std::process::id());
        let fd_list = std::fs::read_dir(fd_list_path).unwrap();
        for fd in fd_list {
            let fd = fd.unwrap();
            let fd = fd.file_name().to_string_lossy().to_string();
            let fd: H = fd.parse().unwrap();
            if fds_to_keep.contains(&fd) {
                continue;
            }
            if -1 == libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) {
                panic!("couldn't cloexec fd: {}", fd);
            }
        }
        //dup2 as late as possible for all panics to write to normal stdio instead of pipes

        libc::dup2(arg.stdin, libc::STDIN_FILENO);
        libc::dup2(arg.stdout, libc::STDOUT_FILENO);
        libc::dup2(arg.stderr, libc::STDERR_FILENO);

        libc::close(arg.stdin);
        libc::close(arg.stdout);
        libc::close(arg.stderr);

        let ret = libc::execve(path, argv as *const *const c_char, envp as *const *const c_char);
        if ret == -1 {
            err_exit("do_exec", "execve");
        }
    }
    0
}

fn setup_pipe(read_end: &mut H, write_end: &mut H) -> Result<()> {
    unsafe {
        let mut sl = [0 as H; 2];
        let ret = libc::pipe(sl.as_mut_ptr());
        if ret == -1 {
            err_exit("setup_pipe", "pipe");
            //return Err(io::Error::last_os_error());
        }
        *read_end = sl[0];
        *write_end = sl[1];
        Ok(())
    }
}

const CHILD_STACK_SIZE: usize = 1024 * 1024;

struct ThreadSafePointer(*mut libc::c_void);

impl ThreadSafePointer {
    //fn from<T>(rf: &mut T) -> ThreadSafePointer {
    //    ThreadSafePointer(rf as *mut c_void)
    //}
}

unsafe impl Sync for ThreadSafePointer {}

unsafe impl Send for ThreadSafePointer {}

pub fn spawn(options: ChildProcessOptions) -> Result<Box<dyn ChildProcess>> {
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
        let child_stack_top = ThreadSafePointer(((child_stack as usize) + CHILD_STACK_SIZE) as
            *mut c_void);
        let dea_ptr = &mut dea as *mut DoExecArg;
        //we need to wrap do_exec process into a thread
        let mut child_pid: c_int = 0;
        let child_pid_box = ThreadSafePointer(&mut child_pid as *mut c_int as *mut c_void);
        let dea_box = ThreadSafePointer(dea_ptr as *mut c_void);
        let thr = thread::spawn(move || {
            //use std::borrow::BorrowMut;=
            let ptr = child_pid_box.0;
            let ptr = ptr as *mut c_int;
            *(ptr.as_mut().unwrap()) =
                libc::clone(do_exec, child_stack_top.0, 0, dea_box.0);
        });
        thr.join().expect("Couldn't join a thread");

        //now we should close handles intended for use by child process
        libc::close(in_r);
        libc::close(out_w);
        libc::close(err_w);

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