//! this module implements a JobServer.
//! JobServer is a long-running process in dominion.
//! JobServer accepts queries for spawning child process

mod setup;

use crate::linux::{
    jail_common::{self, JailOptions},
    pipe::setup_pipe,
    util::{duplicate_string, err_exit, ExitCode, Handle, IpcSocketExt, Pid, Uid},
};
use libc::{c_char, c_void};
use std::{
    ffi::{CString, OsStr, OsString},
    fs, mem,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    ptr, time,
};
use tiny_nix_ipc::Socket;

pub use setup::SetupData;
use std::io::Write;

const SANDBOX_INTERNAL_UID: Uid = 179;

struct Stdio {
    stdin: Handle,
    stdout: Handle,
    stderr: Handle,
}

impl Stdio {
    fn from_fd_array(fds: [Handle; 3]) -> Stdio {
        Stdio {
            stdin: fds[0],
            stdout: fds[1],
            stderr: fds[2],
        }
    }
}

struct JobOptions {
    exe: PathBuf,
    argv: Vec<OsString>,
    env: Vec<OsString>,
    stdio: Stdio,
    pwd: OsString,
}

pub(crate) struct JobServerOptions {
    jail_options: JailOptions,
    sock: Socket,
}

struct DoExecArg {
    //in
    path: OsString,
    arguments: Vec<OsString>,
    environment: Vec<OsString>,
    stdio: Stdio,
    sock: Socket,
    pwd: OsString,
    cgroups_tasks: Vec<Handle>,
}

fn duplicate_string_list(v: &[OsString]) -> *mut *mut c_char {
    let n = v.len();
    let mut res = Vec::with_capacity(n + 1);
    for str in v {
        let str = duplicate_string(&str);
        res.push(str);
    }
    res.push(ptr::null_mut());
    let ret = res.as_mut_ptr();
    mem::forget(res);
    ret
}

const WAIT_MESSAGE_CLASS_EXECVE_PERMITTED: &[u8] = b"EXECVE";

// this function is called, when execve() returned ENOENT, to provide additional information on best effort basis
fn print_diagnostics(path: &OsStr, out: &mut dyn Write) {
    let mut path = std::path::PathBuf::from(path);
    let existing_prefix;
    loop {
        let metadata = fs::metadata(&path);
        if let Ok(m) = metadata {
            if m.is_dir() {
                existing_prefix = path;
                break;
            }
        }
        path.pop();
    }
    writeln!(
        out,
        "following path exists: {:?}, with the following items:",
        &existing_prefix
    )
    .ok();
    let items = fs::read_dir(existing_prefix);
    let items = match items {
        Ok(it) => it,
        Err(e) => {
            writeln!(out, "couldn't list path: {:?}", e).ok();
            return;
        }
    };
    for item in items {
        write!(out, "\t- ").ok();
        match item {
            Ok(item) => {
                writeln!(out, "{}", item.file_name().to_string_lossy()).ok();
            }
            Err(err) => {
                writeln!(out, "<error: {:?}>", err).ok();
            }
        }
    }
}

#[allow(unreachable_code)]
extern "C" fn do_exec(mut arg: DoExecArg) -> ! {
    use std::os::unix::io::FromRawFd;
    unsafe {
        let stderr_fd = libc::dup(2);
        let mut stderr = std::fs::File::from_raw_fd(stderr_fd);
        let path = duplicate_string(&arg.path);

        let mut argv_with_path = vec![arg.path.clone()];
        argv_with_path.append(&mut (arg.arguments.clone()));

        //duplicate argv
        let argv = duplicate_string_list(&argv_with_path);

        //duplicate envp
        let environ = arg.environment.clone();
        let envp = duplicate_string_list(&environ);

        // join cgroups
        // this doesn't require any additional capablities, because we just write some stuff
        // to preopened handle
        let my_pid = std::process::id();
        let my_pid = format!("{}", my_pid);
        for h in arg.cgroups_tasks {
            nix::unistd::write(h, my_pid.as_bytes()).expect("Couldn't join cgroup");
        }

        //now we need mark all FDs as CLOEXEC for not to expose them to sandboxed process
        let fd_list;
        {
            let fd_list_path = "/proc/self/fd".to_string();
            fd_list = fs::read_dir(fd_list_path).unwrap();
        }
        for fd in fd_list {
            let fd = fd.unwrap();
            let fd = fd.file_name().to_string_lossy().to_string();
            let fd: Handle = fd.parse().unwrap();
            if -1 == libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) {
                let fd_info_path = format!("/proc/self/fd/{}", fd);
                let fd_info_path = CString::new(fd_info_path.as_str()).unwrap();
                let mut fd_info = [0 as c_char; 4096];
                libc::readlink(fd_info_path.as_ptr(), fd_info.as_mut_ptr(), 4096);
                let fd_info = CString::from_raw(fd_info.as_mut_ptr());
                let fd_info = fd_info.to_str().unwrap();
                panic!("couldn't cloexec fd: {}({})", fd, fd_info);
            }
        }
        //now let's change our working dir to desired
        let pwd = CString::new(arg.pwd.as_bytes()).unwrap();
        if libc::chdir(pwd.as_ptr()) == -1 {
            let code = nix::errno::errno();
            writeln!(
                stderr,
                "WARNING: couldn't change dir (error {} - {})",
                code,
                nix::errno::from_i32(code).desc()
            )
            .ok();
        }

        if libc::setgid(SANDBOX_INTERNAL_UID as u32) != 0 {
            err_exit("setgid");
        }

        if libc::setuid(SANDBOX_INTERNAL_UID as u32) != 0 {
            err_exit("setuid");
        }
        //now we pause ourselves until parent process places us into appropriate groups
        arg.sock.lock(WAIT_MESSAGE_CLASS_EXECVE_PERMITTED).unwrap();

        //dup2 as late as possible for all panics to write to normal stdio instead of pipes
        libc::dup2(arg.stdio.stdin, libc::STDIN_FILENO);
        libc::dup2(arg.stdio.stdout, libc::STDOUT_FILENO);
        libc::dup2(arg.stdio.stderr, libc::STDERR_FILENO);

        libc::execve(
            path,
            argv as *const *const c_char,
            envp as *const *const c_char,
        );

        let err_code = errno::errno().0;
        if err_code == libc::ENOENT {
            writeln!(
                stderr,
                "FATAL ERROR: executable ({}) was not found",
                &arg.path.to_string_lossy()
            )
            .ok();

            print_diagnostics(&arg.path, &mut stderr);
            libc::exit(108)
        } else {
            writeln!(stderr, "couldn't execute: error code {}", err_code).ok();
            //execve doesn't return on success
            err_exit("execve");
        }
    }
}

unsafe fn spawn_job(
    options: JobOptions,
    setup_data: &SetupData,
) -> crate::Result<jail_common::JobStartupInfo> {
    let (mut sock, mut child_sock) = Socket::new_socketpair().unwrap();
    child_sock
        .no_cloexec()
        .expect("Couldn't make child socket inheritable");
    //will be passed to child process
    let dea = DoExecArg {
        path: options.exe.as_os_str().to_os_string(),
        arguments: options.argv,
        environment: options.env,
        stdio: options.stdio,
        sock: child_sock,
        pwd: options.pwd.clone(),
        cgroups_tasks: setup_data.cgroups.clone(),
    };
    let child_pid: Pid;
    let res = libc::fork();
    if res == -1 {
        err_exit("fork");
    }
    if res == 0 {
        //child
        do_exec(dea);
    }
    //parent
    child_pid = res;

    //now we can allow child to execve()
    sock.wake(WAIT_MESSAGE_CLASS_EXECVE_PERMITTED)?;

    Ok(jail_common::JobStartupInfo { pid: child_pid })
}

const WM_CLASS_SETUP_FINISHED: &[u8] = b"WM_SETUP";
const WM_CLASS_PID_MAP_READY_FOR_SETUP: &[u8] = b"WM_SETUP_READY";
const WM_CLASS_PID_MAP_CREATED: &[u8] = b"WM_PIDMAP_DONE";

mod jobserver_main {
    use crate::linux::{
        jail_common::{JobQuery, Query},
        jobserver::{setup, spawn_job, JobOptions, JobServerOptions, SetupData, Stdio},
        util::{Handle, IpcSocketExt, Pid, StraceLogger},
    };
    use std::{
        ffi::{OsStr, OsString},
        io::Write,
        os::unix::ffi::{OsStrExt, OsStringExt},
        time::Duration,
    };

    fn concat_env_item(k: &OsStr, v: &OsStr) -> OsString {
        let k = k.as_bytes();
        let v = v.as_bytes();
        let cap = k.len() + 1 + v.len();

        let mut res = vec![0; cap];
        res[0..k.len()].copy_from_slice(k);
        res[k.len() + 1..].copy_from_slice(v);
        res[k.len()] = b'=';
        OsString::from_vec(res)
    }

    unsafe fn process_spawn_query(
        arg: &mut JobServerOptions,
        options: &JobQuery,
        setup_data: &SetupData,
    ) -> crate::Result<()> {
        let mut logger = StraceLogger::new();
        write!(logger, "got Spawn request").ok();
        //now we do some preprocessing
        let env: Vec<_> = options
            .environment
            .iter()
            .map(|(k, v)| concat_env_item(OsStr::from_bytes(&base64::decode(k).unwrap()), v))
            .collect();

        let mut child_fds = arg
            .sock
            .recv_struct::<u64, [Handle; 3]>()
            .unwrap()
            .1
            .unwrap();
        for f in child_fds.iter_mut() {
            *f = nix::unistd::dup(*f).unwrap();
        }
        let child_stdio = Stdio::from_fd_array(child_fds);

        let job_options = JobOptions {
            exe: options.image_path.clone(),
            argv: options.argv.clone(),
            env,
            stdio: child_stdio,
            pwd: options.pwd.clone().into_os_string(),
        };

        write!(logger, "JobOptions are fetched").ok();
        let startup_info = spawn_job(job_options, setup_data)?;
        write!(logger, "job started. Sending startup_info back").ok();
        arg.sock.send(&startup_info)?;
        Ok(())
    }

    unsafe fn process_poll_query(
        arg: &mut JobServerOptions,
        pid: Pid,
        timeout: Duration,
    ) -> crate::Result<()> {
        let res = super::timed_wait(pid, timeout)?;
        arg.sock.send(&res)?;
        Ok(())
    }

    pub(crate) unsafe fn jobserver_entry(mut arg: JobServerOptions) -> crate::Result<i32> {
        let setup_data = setup::setup(&arg.jail_options, &mut arg.sock)?;

        let mut logger = StraceLogger::new();
        loop {
            let query: Query = match arg.sock.recv() {
                Ok(q) => {
                    write!(logger, "jobserver: new request").ok();
                    q
                }
                Err(err) => {
                    write!(logger, "jobserver: got unprocessable query: {}", err).ok();
                    return Ok(23);
                }
            };
            match query {
                Query::Spawn(ref o) => process_spawn_query(&mut arg, o, &setup_data)?,
                Query::Exit => break,
                Query::Poll(p) => process_poll_query(&mut arg, p.pid, p.timeout)?,
            };
        }
        Ok(0)
    }
}

struct WaiterArg {
    res_fd: Handle,
    pid: Pid,
}

extern "C" fn kill_me(_code: libc::c_int) {
    unsafe {
        libc::raise(libc::SIGKILL);
    }
}

extern "C" fn timed_wait_waiter(arg: *mut c_void) -> *mut c_void {
    use nix::sys::signal;
    unsafe {
        let arg = arg as *mut WaiterArg;
        let arg = &mut *arg;
        let mut waitstatus = 0;
        {
            let sigaction = signal::SigAction::new(
                signal::SigHandler::Handler(kill_me),
                signal::SaFlags::empty(),
                signal::SigSet::empty(),
            );
            // set SIGILL handler to kill_me()
            signal::sigaction(signal::SIGILL, &sigaction)
                .unwrap_or_else(|_err| err_exit("sigaction"));
        }

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
        //let message_len = message.len();
        let message = CString::new(message).unwrap();
        libc::write(
            arg.res_fd,
            message.as_ptr() as *const _,
            message.as_bytes().len(),
        );
        ptr::null_mut()
    }
}

fn timed_wait(pid: Pid, timeout: time::Duration) -> crate::Result<Option<ExitCode>> {
    unsafe {
        let (mut end_r, mut end_w);
        end_r = 0;
        end_w = 0;
        setup_pipe(&mut end_r, &mut end_w)?;
        //let waiter_stack_layout = alloc::Layout::from_size_align(STACK_SIZE, STACK_ALIGN).unwrap();
        //let waiter_stack = alloc::alloc(waiter_stack_layout);
        let waiter_pid;
        {
            let mut arg = WaiterArg { res_fd: end_w, pid };
            let mut wpid = 0;
            let ret = libc::pthread_create(
                &mut wpid as *mut _,
                ptr::null(),
                timed_wait_waiter,
                &mut arg as *mut WaiterArg as *mut c_void,
            );
            waiter_pid = wpid;
            if ret != 0 {
                errno::set_errno(errno::Errno(ret));
                err_exit("pthread_create");
            }
        }
        //TL&DR - select([ready_r], timeout)
        let mut poll_fd_info: [libc::pollfd; 1];
        poll_fd_info = mem::zeroed();
        let mut poll_fd_ref = &mut poll_fd_info[0];
        poll_fd_ref.fd = end_r;
        poll_fd_ref.events = libc::POLLIN;
        let mut rtimeout: libc::timespec = mem::zeroed();
        rtimeout.tv_sec = timeout.as_secs() as i64;
        rtimeout.tv_nsec = i64::from(timeout.subsec_nanos());
        let ret = loop {
            let poll_ret = libc::ppoll(
                poll_fd_info.as_mut_ptr(),
                1,
                &rtimeout as *const _,
                ptr::null(),
            );
            let ret: Option<_> = match poll_ret {
                -1 => {
                    let sys_err = nix::errno::errno();
                    if sys_err == libc::EINTR {
                        continue;
                    }
                    crate::errors::System { code: sys_err }.fail()?
                }
                0 => None,
                1 => {
                    let mut exit_code = [0; 16];
                    let read_cnt = libc::read(end_r, exit_code.as_mut_ptr() as *mut c_void, 16);
                    if read_cnt == -1 {
                        err_exit("read");
                    }
                    let exit_code =
                        String::from_utf8(exit_code[..read_cnt as usize].to_vec()).unwrap();
                    Some(exit_code.parse().unwrap())
                }
                x => unreachable!("unexpected return code from poll: {}", x),
            };
            break ret;
        };
        libc::pthread_cancel(waiter_pid);
        //alloc::dealloc(waiter_stack, waiter_stack_layout);
        libc::close(end_r);
        libc::close(end_w);
        Ok(ret)
    }
}

pub(crate) unsafe fn start_jobserver(
    jail_options: JailOptions,
) -> crate::Result<jail_common::JobServerStartupInfo> {
    let mut logger = crate::linux::util::strace_logger();
    let (mut sock, js_sock) = Socket::new_socketpair().unwrap();
    let jail_id = jail_common::gen_jail_id();

    let ex_id = format!("/sys/fs/cgroup/pids/jjs/g-{}-ex", &jail_options.jail_id);

    let (return_allowed_r, return_allowed_w) = nix::unistd::pipe().expect("couldn't create pipe");

    let f = libc::fork();
    if f == -1 {
        crate::errors::System {
            code: errno::errno().0,
        }
        .fail()?;
    }

    if f != 0 {
        //thread A: entered start_jobserver() normally, returns from function
        write!(logger, "thread A (main)").unwrap();
        let startup_info = jail_common::JobServerStartupInfo {
            socket: sock,
            wrapper_cgroup_path: OsString::from(ex_id),
        };

        let mut buf = [0 as u8; 4];

        //wait until jobserver is ready
        nix::unistd::read(return_allowed_r, &mut buf).expect("protocol failure");
        nix::unistd::close(return_allowed_r).unwrap();
        nix::unistd::close(return_allowed_w).unwrap();
        return Ok(startup_info);
    }
    // why we use unshare(PID) here, and not in setup_namespace()? See pid_namespaces(7) and unshare(2)
    if libc::unshare(libc::CLONE_NEWPID) == -1 {
        err_exit("unshare");
    }
    let fret = libc::fork();
    if fret == -1 {
        err_exit("fork");
    }
    if fret == 0 {
        //thread C: jobserver main process
        write!(logger, "thread C (jobserver main)").unwrap();
        mem::drop(sock);
        let js_arg = JobServerOptions {
            jail_options,
            sock: js_sock,
        };
        let jobserver_ret_code = jobserver_main::jobserver_entry(js_arg);
        libc::exit(jobserver_ret_code.unwrap_or(1));
    }
    //thread B: external jobserver initializer
    //it's only task currently is pid/gid mapping
    write!(logger, "thread B (jobserver launcher)").unwrap();
    mem::drop(js_sock);
    let child_pid = fret as Pid;

    let sandbox_uid = setup::derive_user_ids(&jail_id);
    // map 0 to 0; map sandbox uid: internal to external
    let mapping = format!("0 0 1\n{} {} 1", SANDBOX_INTERNAL_UID, sandbox_uid);
    let uid_map_path = format!("/proc/{}/uid_map", child_pid);
    let gid_map_path = format!("/proc/{}/gid_map", child_pid);
    sock.lock(WM_CLASS_PID_MAP_READY_FOR_SETUP)?;
    fs::write(&uid_map_path, mapping.as_str()).unwrap();
    fs::write(&gid_map_path, mapping.as_str()).unwrap();
    sock.wake(WM_CLASS_PID_MAP_CREATED)?;
    sock.lock(WM_CLASS_SETUP_FINISHED)?;
    //and now thread A can return
    let wake_buf = [179, 179, 239, 57 /* just magic number */];
    nix::unistd::write(return_allowed_w, &wake_buf).expect("protocol failure");
    libc::exit(0);
}
