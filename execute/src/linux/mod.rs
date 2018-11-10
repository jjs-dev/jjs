mod dominion;
//mod cgroup;

pub use crate::linux::dominion::LinuxDominion;
use libc::{c_int, c_char, c_void};
//use ::definitions::*;
use std::ffi::CString;
use std::{io, sync::{Mutex, Arc, Condvar}, thread, ptr, mem};
use std::time;
use crate::*;

const LINUX_DOMINION_SANITY_CHECK_ID: usize = 0xDEADF00DDEADBEEF;

type H = c_int;
type Pid = libc::pid_t;

fn err_exit(func_name: &str, syscall_name: &str) -> ! {
    let e = errno::errno();
    panic!("{}: {}(2) failed with error {}: {}", func_name, syscall_name, e.0, e);
}


pub struct LinuxReadPipe {
    handle: H,
}

impl std::io::Read for LinuxReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::read(self.handle, buf.as_mut_ptr() as *mut c_void, buf.len());
            if ret == -1 {
                err_exit("LinuxReadPipe::read", "read");
            }
            Ok(ret as usize)
        }
    }
}

impl LinuxReadPipe {
    fn new(handle: H) -> LinuxReadPipe {
        LinuxReadPipe {
            handle
        }
    }
}

pub struct LinuxWritePipe {
    handle: H,
}

impl LinuxWritePipe {
    fn new(handle: H) -> LinuxWritePipe {
        LinuxWritePipe {
            handle
        }
    }
}

impl io::Write for LinuxWritePipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::write(self.handle, buf.as_ptr() as *const c_void, buf.len());
            if ret == -1 {
                err_exit("LinuxWritePipe::write()", "write");
            }
            Ok(ret as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        unsafe {
            let ret = libc::fsync(self.handle);
            if ret == -1 {
                err_exit("LinuxWritePipe::flush", "fsync");
            }
            Ok(())
        }
    }
}

pub struct LinuxChildProcess {
    exit_code: Option<i64>,

    stdin: H,
    stdout: H,
    stderr: H,
    stdio_wasted: bool,

    pid: Pid,
}

macro_rules! SYNC {
($mutex_name:ident) => {

(*$mutex_name).lock().unwrap()

}
}

fn timed_wait(pid: Pid, timeout: time::Duration) -> Option<i32> {
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
            {
                SYNC!(m).exit_code = 228;
                SYNC!(m).exited = true;
                *SYNC!(lock) = true;
                cv.notify_all();
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
            let mut grd = lock.lock().unwrap();
            loop {
                if *grd == true {
                    break;
                }
                grd = cv.wait(grd).unwrap();
            }
        }
        let was_exited = SYNC!(m).exited;
        if !was_exited {
            return None;
        }
        return Some(SYNC!(m).exit_code);
    }
}

impl ChildProcess for LinuxChildProcess {
    type In = LinuxReadPipe;

    type Out = LinuxWritePipe;

    fn get_exit_code(&self) -> Option<i64> {
        self.exit_code
    }

    fn get_stdio(&mut self) -> Option<ChildProcessStdio<LinuxReadPipe, LinuxWritePipe>> {
        match self.stdio_wasted {
            true => None,
            false => {
                self.stdio_wasted = true;
                Some(ChildProcessStdio {
                    stdin: LinuxWritePipe::new(self.stdin),
                    stdout: LinuxReadPipe::new(self.stdout),
                    stderr: LinuxReadPipe::new(self.stderr),
                })
            }
        }
    }

    fn wait_for_exit(&mut self, timeout: std::time::Duration) -> Result<WaitResult> {
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
                    //eprintln!("DEV: marking cp as exited");
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
        return match self.exit_code {
            Some(_) => true,
            None => false,
        };
    }

    fn kill(&mut self) {
        unsafe {
            if self.is_finished() {
                return;
            }
            if libc::kill(self.pid, libc::SIGKILL) == -1 {
                err_exit("LinuxChildProcess::kill", "kill");
            }
        }
    }
}

impl Drop for LinuxChildProcess {
    fn drop(&mut self) {
        if self.is_finished() {
            return;
        }
        //eprintln!("this cp hasn't exited yet");
        self.kill();
        self.wait_for_exit(time::Duration::from_millis(100)).unwrap();
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
    //mutex: *mut libc::pthread_mutex_t,
    ready_fd: H,
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
        if p as usize == 0 {
            panic!("OutOfMemory: malloc returned null");
        }
        ptr::write_bytes(p, 0xDC, num);
        return p;
    }
}

fn allocate_heap_variable<T>() -> *mut T {
    allocate_memory(mem::size_of::<T>()) as *mut T
}

#[allow(unreachable_code)]
extern "C" fn do_exec(arg: *mut c_void) -> i32 {
    use std::iter::FromIterator;
    unsafe {
        let arg = &*(arg as *mut DoExecArg);
        let path = duplicate_string(&arg.path);

        let mut argv_with_path = vec![arg.path.clone()];
        argv_with_path.append(&mut (arg.arguments.clone()));

        //TODO consider refactoring
        //duplicate argv
        let num_argv_items = argv_with_path.len() + 1;
        let argv = allocate_memory(num_argv_items * POINTER_SIZE)
            as *mut *const c_char;
        for (i, argument) in argv_with_path.iter().enumerate() {
            *(argv.offset(i as isize) as *mut *const c_char) = duplicate_string(argument);
        }
        ptr_subscript_set!(argv, num_argv_items-1, ptr::null());

        //duplicate envp
        let num_envp_items = arg.environment.len() + 1;
        let envp = allocate_memory(num_envp_items * POINTER_SIZE)
            as *mut *const c_char;
        for (i, (name, value)) in arg.environment.iter().enumerate() {
            let envp_item = format!("{}={}", name, value);
            *(envp.offset(i as isize) as *mut *const c_char) =
                duplicate_string(&envp_item);
        }
        ptr_subscript_set!(envp, num_envp_items-1, ptr::null());

        //now we need mark all FDs as CLOEXEC for not to expose them to sandboxed process
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

        //some security

        {
            let mut cwd_buf_size = 16;
            let mut cwd_buffer;
            loop {
                cwd_buffer = allocate_memory(cwd_buf_size);
                let ret = libc::getcwd(cwd_buffer, cwd_buf_size);
                if ret as usize == 0 {
                    let err = errno::errno().0;
                    if err == libc::ERANGE {
                        cwd_buf_size *= 2;
                        libc::free(cwd_buffer as *mut _);
                    } else {
                        err_exit("do_exec", "getcwd");
                    }
                } else {
                    break;
                }
            }
            libc::chroot(cwd_buffer);
        }

        if libc::setuid(1001) != 0 { //TODO: hardcoded uid
            err_exit("do_exec", "setuid");
        }
        //now we pause ourselves until parent process places us into appropriate groups
        let mut buf = Vec::with_capacity(READY_MSG.len());
        libc::read(arg.ready_fd, buf.as_mut_ptr() as *mut _, READY_MSG.len());

        //cleanup (empty)

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
        } else {
            unreachable!("execve succeded, but execution of old image continued")
        }
    }
    0
}

fn setup_pipe(read_end: &mut H, write_end: &mut H) -> Result<()> {
    unsafe {
        let mut ends = [0 as H; 2];
        let ret = libc::pipe(ends.as_mut_ptr());
        if ret == -1 {
            err_exit("setup_pipe", "pipe");
            //return Err(io::Error::last_os_error());
        }
        *read_end = ends[0];
        *write_end = ends[1];
        Ok(())
    }
}

const CHILD_STACK_SIZE: usize = 1024 * 1024;

struct ThreadSafePointer(*mut libc::c_void);

unsafe impl Sync for ThreadSafePointer {}

unsafe impl Send for ThreadSafePointer {}

const READY_MSG: &str = "gl_hf";

fn spawn(options: ChildProcessOptions) -> LinuxChildProcess {
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

        let mutex: *mut libc::pthread_mutex_t = allocate_heap_variable();

        if libc::pthread_mutex_init(mutex, ptr::null()) != 0 {
            panic!("pthread error during init\n");
        }

        let mut ready_r = 0;
        let mut ready_w = 0;
        setup_pipe(&mut ready_r, &mut ready_w).unwrap();

        //will be passed to child process
        let mut dea = DoExecArg {
            path: options.path,
            arguments: options.arguments,
            environment: options.environment,
            stdin: in_r,
            stdout: out_w,
            stderr: err_w,
            ready_fd: ready_r,
        };

        let child_stack = libc::malloc(CHILD_STACK_SIZE);
        let child_stack_top = ThreadSafePointer(((child_stack as usize) + CHILD_STACK_SIZE) as
            *mut c_void);
        let dea_ptr = &mut dea as *mut DoExecArg;

        //we need to wrap do_exec call into a thread
        let mut child_pid: c_int = 0;
        let child_pid_box = ThreadSafePointer(&mut child_pid as *mut c_int as *mut c_void);
        let dea_box = ThreadSafePointer(dea_ptr as *mut c_void);
        let thr = thread::spawn(move || {
            let ptr = child_pid_box.0;
            let ptr = ptr as *mut c_int;
            *(ptr.as_mut().unwrap()) =
                libc::clone(do_exec, child_stack_top.0, libc::CLONE_NEWNS,
                            dea_box.0);
        });
        thr.join().expect("Couldn't join a thread");

        //now we should close handles intended for use by child process
        libc::close(in_r);
        libc::close(out_w);
        libc::close(err_w);

        //finally we can set up security
        let dmn = (options.dominion.d as *mut LinuxDominion).as_mut().unwrap();
        //is's unsafe so in case of some error we check it's really LinuxDominion
        //though whole situation is UB, this check doesn't seem to be deleted by compiler
        if dmn.sanity_check() != LINUX_DOMINION_SANITY_CHECK_ID {
            panic!("FATAL ERROR: options.dominion doesn't point on valid LinuxDominion object");
        }
        dmn.add_process(child_pid);

        //now we can allow child to execve()
        libc::write(ready_w, READY_MSG.as_ptr() as *const _, READY_MSG.len());

        LinuxChildProcess {
            exit_code: None,
            stdin: in_w,
            stdout: out_r,
            stderr: err_r,
            stdio_wasted: false,
            pid: child_pid,
        }
    }
}

pub struct LinuxEM {}


impl ExecutionManager for LinuxEM {
    type ChildProcess = LinuxChildProcess;
    fn new_dominion(&mut self, options: DominionOptions) -> DominionRef {
        let d = linux::dominion::LinuxDominion::create(options);
        DominionRef {
            d,
        }
    }

    fn spawn(&mut self, options: ChildProcessOptions) -> LinuxChildProcess {
        spawn(options)
    }
}

pub fn setup_execution_manager() -> LinuxEM {
    LinuxEM {}
}