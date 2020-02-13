//! this module implements a Zygote.
//! Jygote is a long-running process in dominion.
//! In particular, zygote is namespace root.
//! Zygote accepts queries for spawning child process

mod main_loop;
mod setup;
pub(in crate::linux) mod cgroup;

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

use setup::SetupData;
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

pub(crate) struct ZygoteOptions {
    jail_options: JailOptions,
    sock: Socket,
}

struct DoExecArg {
    path: OsString,
    arguments: Vec<OsString>,
    environment: Vec<OsString>,
    stdio: Stdio,
    sock: Socket,
    pwd: OsString,
    cgroups_tasks: cgroup::Group,
    jail_id: String,
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

// This function is called, when execve() returned ENOENT, to provide additional information on best effort basis.
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

extern "C" fn do_exec(mut arg: DoExecArg) -> ! {
    use std::os::unix::io::FromRawFd;
    unsafe {
        let stderr_fd = libc::dup(2);
        let mut stderr = std::fs::File::from_raw_fd(stderr_fd);
        let path = duplicate_string(&arg.path);

        let mut argv_with_path = vec![arg.path.clone()];
        argv_with_path.append(&mut (arg.arguments.clone()));

        // Duplicate argv.
        let argv = duplicate_string_list(&argv_with_path);

        // Duplicate envp.
        let environ = arg.environment.clone();
        let envp = duplicate_string_list(&environ);

        // Join cgroups.
        // This doesn't require any additional capablities, because we just write some stuff
        // to preopened handle.
        /*for h in arg.cgroups_tasks {
        }*/
        arg.cgroups_tasks.join_self();

        // Now we need mark all FDs as CLOEXEC for not to expose them to sandboxed process
        let fd_list;
        {
            let fd_list_path = "/proc/self/fd".to_string();
            fd_list = fs::read_dir(fd_list_path).expect("failed to enumerate /proc/self/fd");
        }
        for fd in fd_list {
            let fd = fd.expect("failed to get fd entry metadata");
            let fd = fd.file_name().to_string_lossy().to_string();
            let fd: Handle = fd
                .parse()
                .expect("/proc/self/fd member file name is not fd");
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
        // Now let's change our working dir to desired.
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
            // It is not error from security PoV if chdir failed: chroot isolation works even if current dir is outside of chroot.
        }

        if libc::setgid(SANDBOX_INTERNAL_UID as u32) != 0 {
            err_exit("setgid");
        }

        if libc::setuid(SANDBOX_INTERNAL_UID as u32) != 0 {
            err_exit("setuid");
        }
        // Now we pause ourselves until parent process places us into appropriate groups.
        arg.sock.lock(WAIT_MESSAGE_CLASS_EXECVE_PERMITTED).unwrap();

        // Call dup2 as late as possible for all panics to write to normal stdio instead of pipes.
        libc::dup2(arg.stdio.stdin, libc::STDIN_FILENO);
        libc::dup2(arg.stdio.stdout, libc::STDOUT_FILENO);
        libc::dup2(arg.stdio.stderr, libc::STDERR_FILENO);

        let mut logger = crate::linux::util::StraceLogger::new();
        writeln!(logger, "dominion {}: ready to execve", arg.jail_id).unwrap();

        libc::execve(
            path,
            argv as *const *const c_char,
            envp as *const *const c_char,
        );
        // Execve only returns on error.

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
            err_exit("execve");
        }
    }
}

unsafe fn spawn_job(
    options: JobOptions,
    setup_data: &SetupData,
    jail_id: String,
) -> crate::Result<jail_common::JobStartupInfo> {
    let (mut sock, mut child_sock) = Socket::new_socketpair().unwrap();
    child_sock
        .no_cloexec()
        .expect("Couldn't make child socket inheritable");
    // `dea` will be passed to child process
    let dea = DoExecArg {
        path: options.exe.as_os_str().to_os_string(),
        arguments: options.argv,
        environment: options.env,
        stdio: options.stdio,
        sock: child_sock,
        pwd: options.pwd.clone(),
        cgroups_tasks: setup_data.cgroups.clone(),
        jail_id,
    };
    let child_pid: Pid;
    let res = libc::fork();
    if res == -1 {
        err_exit("fork");
    }
    if res == 0 {
        // Child
        do_exec(dea);
    }
    // Parent
    child_pid = res;

    // Now we can allow child to execve()
    sock.wake(WAIT_MESSAGE_CLASS_EXECVE_PERMITTED)?;

    Ok(jail_common::JobStartupInfo { pid: child_pid })
}

const WM_CLASS_SETUP_FINISHED: &[u8] = b"WM_SETUP";
const WM_CLASS_PID_MAP_READY_FOR_SETUP: &[u8] = b"WM_SETUP_READY";
const WM_CLASS_PID_MAP_CREATED: &[u8] = b"WM_PIDMAP_DONE";

struct WaiterArg {
    res_fd: Handle,
    pid: Pid,
}

extern "C" fn timed_wait_waiter(arg: *mut c_void) -> *mut c_void {
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
        // TL&DR - select([ready_r], timeout)
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
        libc::close(end_r);
        libc::close(end_w);
        Ok(ret)
    }
}

pub(crate) unsafe fn start_zygote(
    jail_options: JailOptions,
) -> crate::Result<jail_common::ZygoteStartupInfo> {
    let mut logger = crate::linux::util::strace_logger();
    let (mut sock, js_sock) = Socket::new_socketpair().unwrap();
    let jail_id = jail_common::gen_jail_id();

    let (return_allowed_r, return_allowed_w) = nix::unistd::pipe().expect("couldn't create pipe");

    let f = libc::fork();
    if f == -1 {
        crate::errors::System {
            code: errno::errno().0,
        }
        .fail()?;
    }

    if f != 0 {
        // Thread A it is thread that entered start_zygote() normally, returns from function
        write!(
            logger,
            "dominion {}: thread A (main)",
            &jail_options.jail_id
        )
        .unwrap();

        let mut zygote_pid_bytes = [0 as u8; 4];

        // Wait until zygote is ready.
        // Zygote is ready when zygote launcher returns it's pid
        nix::unistd::read(return_allowed_r, &mut zygote_pid_bytes).expect("protocol violation");
        nix::unistd::close(return_allowed_r).unwrap();
        nix::unistd::close(return_allowed_w).unwrap();
        nix::unistd::close(jail_options.watchdog_chan).unwrap();
        let startup_info = jail_common::ZygoteStartupInfo {
            socket: sock,
            zygote_pid: i32::from_ne_bytes(zygote_pid_bytes),
        };
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
        // Thread C is zygote main process
        write!(
            logger,
            "dominion {}: thread C (zygote main)",
            &jail_options.jail_id
        )
        .unwrap();
        mem::drop(sock);
        let js_arg = ZygoteOptions {
            jail_options,
            sock: js_sock,
        };
        let zygote_ret_code = main_loop::zygote_entry(js_arg);
        libc::exit(zygote_ret_code.unwrap_or(1));
    }
    // Thread B is external zygote initializer.
    // It's only task currently is to setup uid/gid mapping.
    write!(
        logger,
        "dominion {}: thread B (zygote launcher)",
        &jail_options.jail_id
    )
    .unwrap();
    mem::drop(js_sock);
    let child_pid = fret as Pid;

    let sandbox_uid = setup::derive_user_ids(&jail_id);
    // Map 0 to 0; map sandbox uid: internal to external.
    let mapping = format!("0 0 1\n{} {} 1", SANDBOX_INTERNAL_UID, sandbox_uid);
    let uid_map_path = format!("/proc/{}/uid_map", child_pid);
    let gid_map_path = format!("/proc/{}/gid_map", child_pid);
    sock.lock(WM_CLASS_PID_MAP_READY_FOR_SETUP)?;
    fs::write(&uid_map_path, mapping.as_str()).unwrap();
    fs::write(&gid_map_path, mapping.as_str()).unwrap();
    sock.wake(WM_CLASS_PID_MAP_CREATED)?;
    sock.lock(WM_CLASS_SETUP_FINISHED)?;
    // And now thread A can return.
    nix::unistd::write(return_allowed_w, &child_pid.to_ne_bytes()).expect("protocol violation");
    libc::exit(0);
}
