mod dominion;
mod util;

pub use crate::linux::dominion::LinuxDominion;
use crate::linux::util::{dev_log, err_exit, setup_pipe, Handle, Pid, Sock};
use crate::*;
use libc::{c_char, c_int, c_void};
use std::{
    ffi::CString,
    fs, io, mem, ptr,
    sync::{Arc, Mutex},
    time,
};

const LINUX_DOMINION_SANITY_CHECK_ID: u64 = 0xDEAD_F00D_DEAD_BEEF;

pub struct LinuxReadPipe {
    handle: Handle,
}

impl std::io::Read for LinuxReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::read(self.handle, buf.as_mut_ptr() as *mut c_void, buf.len());
            if ret == -1 {
                err_exit("read");
            }
            Ok(ret as usize)
        }
    }
}

impl LinuxReadPipe {
    fn new(handle: Handle) -> LinuxReadPipe {
        LinuxReadPipe { handle }
    }
}

pub struct LinuxWritePipe {
    handle: Handle,
}

impl LinuxWritePipe {
    fn new(handle: Handle) -> LinuxWritePipe {
        LinuxWritePipe { handle }
    }
}

impl io::Write for LinuxWritePipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::write(self.handle, buf.as_ptr() as *const c_void, buf.len());
            if ret == -1 {
                let ret_code = errno::errno().0;
                return Err(io::Error::last_os_error())
            }
            Ok(ret as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        unsafe {
            let ret = libc::fsync(self.handle);
            if ret == -1 {
                return Err(io::Error::last_os_error())
            }
            Ok(())
        }
    }
}

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

extern "C" fn timed_wait_waiter(arg: *mut c_void) -> i32 {
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

fn timed_wait(pid: Pid, timeout: time::Duration) -> Option<i32> {
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

    fn wait_for_exit(&mut self, timeout: std::time::Duration) -> Result<WaitResult> {
        unsafe {
            if self.is_finished() {
                return Ok(WaitResult::AlreadyFinished);
            }
            let wait_result = timed_wait(self.pid, timeout);
            match wait_result {
                None => Ok(WaitResult::Timeout),
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

struct DoExecArg {
    //in
    path: String,
    arguments: Vec<String>,
    environment: std::collections::HashMap<String, String>,
    stdin: Handle,
    stdout: Handle,
    stderr: Handle,
    root: String,
    dominion: *mut LinuxDominion,
    sock: Sock,
}

const WAIT_MESSAGE_CLASS_EXECVE_PERMITTED: u16 = 1;

fn duplicate_string_vec(v: &[String]) -> *mut *mut c_char {
    let n = v.len();
    let mut res = Vec::with_capacity(n + 1);
    for str in v {
        let str = util::duplicate_string(str.as_str());
        res.push(str);
    }
    res.push(ptr::null_mut());
    let p = res.as_ptr();
    let byte_cnt = (n + 1) * mem::size_of::<*mut c_char>();
    let res = util::allocate_memory(byte_cnt);
    unsafe {
        libc::memcpy(res as *mut c_void, p as *const c_void, byte_cnt);
    }
    res as *mut *mut c_char
}

#[allow(unreachable_code)]
extern "C" fn do_exec(arg: *mut c_void) -> i32 {
    use std::iter::FromIterator;
    unsafe {
        dev_log("do_exec()");
        let arg = &mut *(arg as *mut DoExecArg);
        let path = util::duplicate_string(&arg.path);

        let mut argv_with_path = vec![arg.path.clone()];
        argv_with_path.append(&mut (arg.arguments.clone()));

        //duplicate argv
        let argv = duplicate_string_vec(&arg.arguments);

        //duplicate envp
        let environ: Vec<String> = arg
            .environment
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let envp = duplicate_string_vec(&environ);

        let dominion = &mut *arg.dominion;
        //before chroot we will perform all required mounts
        dominion.expose_dirs();

        //enter namespaces
        dominion.enter();

        //TODO move to dominion
        //change root (we assume all required binaries, DLLs etc will be made available via expose)
        {
            let new_root = CString::new(arg.root.as_str()).unwrap();
            if -1 == libc::chroot(new_root.as_ptr()) {
                err_exit("chroot");
            }
        }

        //now we need mark all FDs as CLOEXEC for not to expose them to sandboxed process
        let fds_to_keep = vec![arg.stdin, arg.stdout, arg.stderr];
        let fds_to_keep = std::collections::BTreeSet::from_iter(fds_to_keep.iter());
        let fd_list;
        {
            let fd_list_path = "/proc/self/fd".to_string();
            fd_list = fs::read_dir(fd_list_path).unwrap();
        }
        for fd in fd_list {
            let fd = fd.unwrap();
            let fd = fd.file_name().to_string_lossy().to_string();
            let fd: Handle = fd.parse().unwrap();
            if fds_to_keep.contains(&fd) {
                continue;
            }
            if -1 == libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) {
                let fd_info_path = format!("/proc/self/fd/{}", fd);
                let fd_info_path = CString::new(fd_info_path.as_str()).unwrap();
                let mut fd_info = [0 as i8; 4096];
                libc::readlink(fd_info_path.as_ptr(), fd_info.as_mut_ptr(), 4096);
                let fd_info = CString::from_raw(fd_info.as_mut_ptr());
                let fd_info = fd_info.to_str().unwrap();
                panic!("couldn't cloexec fd: {}({})", fd, fd_info);
            }
        }

        let sandbox_user_id = 1; //thanks to /proc/self/uid_map
        if libc::setuid(sandbox_user_id as u32) != 0 {
            err_exit("setuid");
        }
        //now we pause ourselves until parent process places us into appropriate groups
        {
            let permission: util::WaitMessage = arg.sock.receive();
            permission.check(WAIT_MESSAGE_CLASS_EXECVE_PERMITTED);
        }

        //cleanup (empty)

        //dup2 as late as possible for all panics to write to normal stdio instead of pipes
        libc::dup2(arg.stdin, libc::STDIN_FILENO);
        libc::dup2(arg.stdout, libc::STDOUT_FILENO);
        libc::dup2(arg.stderr, libc::STDERR_FILENO);

        //we close these FDs because they weren't affected by FD_CLOEXEC
        libc::close(arg.stdin);
        libc::close(arg.stdout);
        libc::close(arg.stderr);

        libc::execve(
            path,
            argv as *const *const c_char,
            envp as *const *const c_char,
        );
        //execve doesn't return on success
        err_exit("execve");
    }
}

fn spawn(options: ChildProcessOptions) -> LinuxChildProcess {
    unsafe {
        let dmn = (options.dominion.d.lock().unwrap().ptr).as_mut().unwrap();
        //is's unsafe so in case of some error we check it's really LinuxDominion
        //though whole situation is UB, this check doesn't seem to be deleted by compiler
        if dmn.sanity_check() != LINUX_DOMINION_SANITY_CHECK_ID {
            panic!("[libminion] FATAL ERROR: options.dominion doesn't point on valid LinuxDominion object");
        }

        let mut in_r = 0;
        let mut in_w = 0;
        let mut out_r = 0;
        let mut out_w = 0;
        let mut err_r = 0;
        let mut err_w = 0;

        setup_pipe(&mut in_r, &mut in_w);
        setup_pipe(&mut out_r, &mut out_w);
        setup_pipe(&mut err_r, &mut err_w);

        let root = dmn.dir();
        let (mut sock, child_sock) = Sock::make_pair();
        //will be passed to child process
        let mut dea = DoExecArg {
            path: options.path,
            arguments: options.arguments,
            environment: options.environment,
            stdin: in_r,
            stdout: out_w,
            stderr: err_w,
            sock: child_sock,
            root,
            dominion: options.dominion.d.lock().unwrap().ptr, //uff
        };

        let dea_ptr = &mut dea as *mut DoExecArg;

        let mut child_pid: c_int = 0;

        let res = libc::fork();
        if res == -1 {
            err_exit("fork");
        }
        if res == 0 {
            //child
            do_exec(dea_ptr as *mut _);
        } else {
            //parent
            child_pid = res;
        }

        //now we should close handles intended for use by child process
        libc::close(in_r);
        libc::close(out_w);
        libc::close(err_w);

        //finally we can set up security
        dev_log("adding process into dominion");
        dmn.add_process(child_pid);

        //now we can allow child to execve()
        sock.send(&util::WaitMessage::with_class(
            WAIT_MESSAGE_CLASS_EXECVE_PERMITTED,
        ));

        LinuxChildProcess {
            exit_code: None,
            stdin: in_w,
            stdout: out_r,
            stderr: err_r,
            stdio_wasted: false,
            pid: child_pid,
            _dominion_ref: options.dominion.clone(),
        }
    }
}

pub struct LinuxBackend {}

impl Backend for LinuxBackend {
    type ChildProcess = LinuxChildProcess;
    fn new_dominion(&mut self, options: DominionOptions) -> DominionRef {
        let pd = linux::dominion::LinuxDominion::create(options);
        DominionRef {
            d: Arc::new(Mutex::new(DominionPointerOwner { ptr: pd })),
        }
    }

    fn spawn(&mut self, options: ChildProcessOptions) -> LinuxChildProcess {
        spawn(options)
    }
}

fn empty_signal_handler(_1: libc::c_int, _2: *mut libc::siginfo_t, _3: *mut libc::c_void) {}

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
