//! this module implements a JobServer.
//! JobServer is a long-running process in dominion.
//! JobServer accepts queries for spawning child process

use crate::{
    linux::{
        dominion::DesiredAccess,
        jail_common::{self, get_path_for_subsystem, JailOptions},
        pipe::setup_pipe,
        util::{duplicate_string, err_exit, ExitCode, Handle, IpcSocketExt, Pid, Uid},
    },
    PathExpositionOptions,
};
use libc::{c_char, c_void};
use std::{
    alloc, collections::hash_map::DefaultHasher, ffi::CString, fs, hash::Hasher, io, mem, process,
    ptr, time,
};
use tiny_nix_ipc::Socket;

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
    exe: String,
    argv: Vec<String>,
    env: Vec<String>,
    stdio: Stdio,
    pwd: String,
}

pub(crate) struct JobServerOptions {
    jail_options: JailOptions,
    sock: Socket,
}

struct DoExecArg {
    //in
    path: String,
    arguments: Vec<String>,
    environment: Vec<String>,
    stdio: Stdio,
    sock: Socket,
    pwd: String,
    cgroups_tasks: Vec<Handle>,
}

fn get_mount_target(base: &str, suf: &str) -> String {
    let mut suf: String = suf.into();
    if suf.starts_with('/') {
        suf = suf[1..].into();
    }
    if suf.ends_with('/') {
        suf.pop();
    }
    let mut dir = base.to_string();
    if dir.ends_with('/') {
        dir.pop();
    }
    return format!("{}/{}", dir, suf);
}

unsafe fn configure_dir(dir_path: &str, uid: Uid) {
    let mode = libc::S_IRUSR
        | libc::S_IWUSR
        | libc::S_IXUSR
        | libc::S_IRGRP
        | libc::S_IWGRP
        | libc::S_IXGRP
        | libc::S_IROTH
        | libc::S_IWOTH
        | libc::S_IXOTH;
    let path = CString::new(dir_path).unwrap();
    if libc::chmod(path.clone().as_ptr(), mode) == -1 {
        err_exit("chmod");
    }

    if libc::chown(path.clone().as_ptr(), uid, uid) == -1 {
        err_exit("chown");
    }
}

fn expose_dir(
    jail_root: &str,
    system_path: &str,
    alias_path: &str,
    access: DesiredAccess,
    uid: Uid,
) {
    let bind_target = get_mount_target(jail_root, alias_path);
    fs::create_dir_all(&bind_target).unwrap();
    let orig_bind_target = bind_target.clone();
    let bind_target = CString::new(bind_target).unwrap();
    let bind_src = CString::new(system_path).unwrap();
    unsafe {
        let mnt_res = libc::mount(
            bind_src.as_ptr(),
            bind_target.clone().as_ptr(),
            ptr::null(),
            libc::MS_BIND,
            ptr::null(),
        );
        if mnt_res == -1 {
            err_exit("mount");
        }

        configure_dir(&orig_bind_target, uid);

        if let DesiredAccess::Readonly = access {
            let rem_ret = libc::mount(
                ptr::null(),
                bind_target.clone().as_ptr(),
                ptr::null(),
                libc::MS_BIND | libc::MS_REMOUNT | libc::MS_RDONLY,
                ptr::null(),
            );
            if rem_ret == -1 {
                err_exit("mount");
            }
        }
    }
}

pub(crate) fn expose_dirs(expose: &[PathExpositionOptions], jail_root: &str, uid: Uid) {
    //mount --bind
    for x in expose {
        expose_dir(jail_root, &x.src, &x.dest, x.access.clone(), uid)
    }
}

fn duplicate_string_list(v: &[String]) -> *mut *mut c_char {
    let n = v.len();
    let mut res = Vec::with_capacity(n + 1);
    for str in v {
        let str = duplicate_string(str.as_str());
        res.push(str);
    }
    res.push(ptr::null_mut());
    let ret = res.as_mut_ptr();
    mem::forget(res);
    ret
}

const WAIT_MESSAGE_CLASS_EXECVE_PERMITTED: u16 = 1;

// this function is called, when execve() returned ENOENT, to provide additional information on best effort basis
fn print_diagnostics(path: &str) {
    use std::str::FromStr;
    let path = std::path::PathBuf::from_str(path);
    let mut path = match path {
        Ok(p) => p,
        Err(e) => {
            eprintln!("couldn't parse path: {:?}", e);
            return;
        }
    };
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
    eprintln!(
        "following path exists: {:?}, with the following items:",
        &existing_prefix
    );
    let items = fs::read_dir(existing_prefix);
    let items = match items {
        Ok(it) => it,
        Err(e) => {
            eprintln!("couldn't list path: {:?}", e);
            return;
        }
    };
    for item in items {
        eprint!("\t- ");
        match item {
            Ok(item) => {
                eprintln!("{}", item.file_name().to_string_lossy());
            }
            Err(err) => {
                eprintln!("<error: {:?}>", err);
            }
        }
    }
}

#[allow(unreachable_code)]
extern "C" fn do_exec(mut arg: DoExecArg) -> ! {
    unsafe {
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
        let pwd = CString::new(arg.pwd).unwrap();
        if libc::chdir(pwd.as_ptr()) == -1 {
            let code = nix::errno::errno();
            eprintln!(
                "WARNING: couldn't change dir (error {} - {})",
                code,
                nix::errno::from_i32(code).desc()
            );
        }


        if libc::setuid(SANDBOX_INTERNAL_UID as u32) != 0 {
            err_exit("setuid");
        }
        if libc::setgid(SANDBOX_INTERNAL_UID as u32) != 0 {
            err_exit("setgid");
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
            eprintln!("FATAL ERROR: executable ({}) was not found", &arg.path);

            print_diagnostics(&arg.path);
            libc::exit(108)
        } else {
            eprintln!("couldn't execute: error code {}", err_code);
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
        path: options.exe,
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

const WM_CLASS_SETUP_FINISHED: u16 = 1;
const WM_CLASS_PID_MAP_READY_FOR_SETUP: u16 = 2;
const WM_CLASS_PID_MAP_CREATED: u16 = 3;

struct SetupData {
    cgroups: Vec<Handle>,
}

extern "C" fn sigterm_handler(_signal: i32) {
    unsafe { libc::exit(0) }
}

unsafe fn setup_sighandler() {
    use nix::sys::signal;
    // SIGTERM
    {
        let handler = signal::SigHandler::Handler(sigterm_handler);
        let action =
            signal::SigAction::new(handler, signal::SaFlags::empty(), signal::SigSet::empty());
        signal::sigaction(signal::Signal::SIGTERM, &action).expect("Couldn't setup sighandler");
    }
}

unsafe fn setup_cgroups(jail_options: &JailOptions) -> Vec<Handle> {
    let jail_id = jail_options.jail_id.clone();
    //configure cpuacct subsystem
    let cpuacct_cgroup_path = get_path_for_subsystem("cpuacct", &jail_id);
    fs::create_dir_all(&cpuacct_cgroup_path).unwrap();

    //configure pids subsystem
    let pids_cgroup_path = get_path_for_subsystem("pids", &jail_id);
    fs::create_dir_all(&pids_cgroup_path).unwrap();

    fs::write(
        format!("{}/pids.max", &pids_cgroup_path),
        format!("{}", jail_options.max_alive_process_count),
    )
        .unwrap();

    //configure memory subsystem
    let mem_cgroup_path = get_path_for_subsystem("memory", &jail_id);

    fs::create_dir_all(&mem_cgroup_path).unwrap();
    fs::write(format!("{}/memory.swappiness", &mem_cgroup_path), "0").unwrap();

    fs::write(
        format!("{}/memory.limit_in_bytes", &mem_cgroup_path),
        format!("{}", jail_options.memory_limit),
    )
        .unwrap();

    let my_pid: Pid = libc::getpid();
    if my_pid == -1 {
        err_exit("getpid");
    }
    // now we setup additional pids cgroup
    // it will only be used for killing all the dominion
    let add_id = format!("{}-ex", jail_id);
    {
        let additional_pids = get_path_for_subsystem("pids", &add_id);
        fs::create_dir_all(&additional_pids).unwrap();
        let add_tasks_file = format!("{}/tasks", &additional_pids);
        fs::write(add_tasks_file, format!("{}", my_pid)).unwrap();
    }

    // we return handles to tasksfiles for main cgroups
    // so, though jobserver itself and children are in chroot, and cannot access cgroupfs, they will be able to add themselves to cgroups
    ["cpuacct", "pids", "memory"]
        .iter()
        .map(|subsys_name| {
            use std::os::unix::io::IntoRawFd;
            let p = get_path_for_subsystem(subsys_name, &jail_id);
            let p = format!("{}/tasks", p);
            let h = fs::OpenOptions::new()
                .write(true)
                .open(p)
                .expect("Couldn't open tasks file")
                .into_raw_fd();
            libc::dup(h)
        })
        .collect::<Vec<_>>()
}

unsafe fn setup_namespaces(_jail_options: &JailOptions) {
    if libc::unshare(/*FIXME: libc::CLONE_NEWNET |*/ libc::CLONE_NEWUSER) == -1 {
        err_exit("unshare")
    }
}

unsafe fn setup_chroot(jail_options: &JailOptions) {
    let path = jail_options.isolation_root.clone();
    let path = CString::new(path.as_str()).unwrap();
    if libc::chroot(path.as_ptr()) == -1 {
        err_exit("chroot");
    }
    let path_root = CString::new("/").unwrap();
    if libc::chdir(path_root.as_ptr()) == -1 {
        err_exit("chdir");
    }
}

unsafe fn setup_procfs(jail_options: &JailOptions) {
    let procfs_path = format!("{}/proc", jail_options.isolation_root.as_str());
    match fs::create_dir(procfs_path.as_str()) {
        Ok(_) => (),
        Err(e) => match e.kind() {
            io::ErrorKind::AlreadyExists => (),
            _ => Err(e).unwrap(),
        },
    }
    let proc = CString::new("proc").unwrap();
    let targ = CString::new(procfs_path.as_str()).unwrap();
    let mret = libc::mount(
        proc.clone().as_ptr(),
        targ.clone().as_ptr(),
        proc.clone().as_ptr(),
        0,
        ptr::null(),
    );
    if -1 == mret {
        err_exit("mount")
    }
}

unsafe fn setup_uid_mapping(sock: &mut Socket) -> crate::Result<()> {
    sock.wake(WM_CLASS_PID_MAP_READY_FOR_SETUP)?;
    sock.lock(WM_CLASS_PID_MAP_CREATED)?;
    Ok(())
}

unsafe fn setup_time_watch(jail_options: &JailOptions) -> crate::Result<()> {
    let cpu_tl = jail_options.time_limit.as_nanos() as u64;
    let real_tl = cpu_tl * 3; //TODO to JailOptions
    observe_time(&jail_options.jail_id, cpu_tl, real_tl)
}


///derives user_ids (in range 1_000_000 to 2_000_000) from jail_id in determined way
fn derive_user_ids(jail_id: &str) -> Uid {
    let jail_id = jail_id.as_bytes();
    let mut hasher = DefaultHasher::new();
    hasher.write(jail_id);
    (hasher.finish() % 2_000_000 + 1_000_000) as Uid
}

unsafe fn setup_expositions(options: &JailOptions, uid: Uid) {
    expose_dirs(&options.exposed_paths, &options.isolation_root, uid);
}

unsafe fn setup(jail_params: &JailOptions, sock: &mut Socket) -> crate::Result<SetupData> {
    let uid = derive_user_ids(&jail_params.jail_id);
    configure_dir(&jail_params.isolation_root, uid);
    setup_sighandler();
    setup_expositions(&jail_params, uid);
    setup_procfs(&jail_params);
    let handles = setup_cgroups(&jail_params);
    //it's important cpu watcher will be outside of user namespace
    setup_time_watch(&jail_params)?;
    setup_namespaces(&jail_params);
    setup_uid_mapping(sock)?;
    setup_chroot(&jail_params);
    sock.wake(WM_CLASS_SETUP_FINISHED)?;
    let res = SetupData { cgroups: handles };
    Ok(res)
}

mod jobserver_main {
    use crate::linux::{
        jail_common::{JobQuery, Query},
        jobserver::{setup, spawn_job, JobOptions, JobServerOptions, SetupData, Stdio},
        util::{Handle, IpcSocketExt, Pid},
    };
    use std::time::Duration;

    unsafe fn process_spawn_query(
        arg: &mut JobServerOptions,
        options: &JobQuery,
        setup_data: &SetupData,
    ) -> crate::Result<()> {
        //now we do some preprocessing

        let env: Vec<_> = options
            .environment
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
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
            pwd: options.pwd.clone(),
        };

        let startup_info = spawn_job(job_options, setup_data)?;
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
        let setup_data = setup(&arg.jail_options, &mut arg.sock)?;

        loop {
            let query: Query = arg.sock.recv().unwrap();
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

const STACK_SIZE: usize = (1 << 20);
//one megabyte
const STACK_ALIGN: usize = (1 << 4); //16 bytes, as required by SysV-64 ABI

fn timed_wait(pid: Pid, timeout: time::Duration) -> crate::Result<Option<ExitCode>> {
    unsafe {
        let (mut end_r, mut end_w);
        end_r = 0;
        end_w = 0;
        setup_pipe(&mut end_r, &mut end_w)?;
        let waiter_stack_layout = alloc::Layout::from_size_align(STACK_SIZE, STACK_ALIGN).unwrap();
        let waiter_stack = alloc::alloc(waiter_stack_layout);
        let waiter_pid;
        {
            let mut arg = WaiterArg { res_fd: end_w, pid };
            //let argp = util::allocate_heap_variable();
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
                    return Err(crate::ErrorKind::System(sys_err).into());
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
        alloc::dealloc(waiter_stack, waiter_stack_layout);
        libc::close(end_r);
        libc::close(end_w);
        Ok(ret)
    }
}

//internal function, kills processes which used all their CPU time limit
//timings are given in nanoseconds
unsafe fn cpu_time_observer(jail_id: &str, cpu_time_limit: u64, real_time_limit: u64) -> ! {
    let start = time::Instant::now();
    loop {
        libc::sleep(1);
        let current_usage_file = jail_common::get_path_for_subsystem("cpuacct", jail_id);
        let current_usage_file = format!("{}/cpuacct.usage", current_usage_file);
        let current_usage = fs::read_to_string(current_usage_file)
            .expect("Couldn't load cpu usage")
            .trim()
            .parse::<u64>()
            .unwrap();
        let elapsed = time::Instant::now().duration_since(start);
        let elapsed = elapsed.as_nanos();
        let was_cpu_tle = current_usage > cpu_time_limit;
        let was_real_tle = elapsed as u64 > real_time_limit;
        let ok = !was_cpu_tle && !was_real_tle;
        if ok {
            continue;
        }
        let my_pid = process::id();
        jail_common::cgroup_kill_all(jail_id, Some(my_pid as Pid)).unwrap();
        break;
    }
    libc::exit(0)
}

unsafe fn observe_time(
    jail_id: &str,
    cpu_time_limit: u64,
    real_time_limit: u64,
) -> crate::Result<()> {
    let fret = libc::fork();
    if fret == -1 {
        Err(crate::ErrorKind::System(nix::errno::errno()))?;
    }
    if fret == 0 {
        cpu_time_observer(jail_id, cpu_time_limit, real_time_limit)
    } else {
        Ok(())
    }
}

pub(crate) unsafe fn start_jobserver(
    jail_options: JailOptions,
) -> crate::Result<jail_common::JobServerStartupInfo> {
    use std::io::Write;
    let mut logger = crate::linux::util::strace_logger();
    let (mut sock, js_sock) = Socket::new_socketpair().unwrap();
    let jail_id = jail_common::gen_jail_id();

    let ex_id = format!("/sys/fs/cgroup/pids/jjs/g-{}-ex", &jail_options.jail_id);

    let (return_allowed_r, return_allowed_w) = nix::unistd::pipe().expect("couldn't create pipe");

    let f = libc::fork();
    if f == -1 {
        return Err(crate::ErrorKind::System(errno::errno().0).into());
    }

    if f != 0 {
        //thread A: entered start_jobserver() normally, returns from function
        write!(logger, "thread A (main)").unwrap();
        let startup_info = jail_common::JobServerStartupInfo {
            socket: sock,
            wrapper_cgroup_path: ex_id,
        };

        let mut buf = [0 as u8; 1];

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
            jail_options: jail_options.clone(),
            sock: js_sock,
        };
        let jobserver_ret_code = jobserver_main::jobserver_entry(js_arg);
        libc::exit(jobserver_ret_code.unwrap_or(1));
    }
    //thread B: external jobserver initializer
    //it's only task currently is pid/gid mapping
    write!(logger, "thread C (jobserver launcher)").unwrap();
    mem::drop(js_sock);
    let child_pid = fret as Pid;

    let sandbox_uid = derive_user_ids(&jail_id);
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
    let wake_buf = [179, 179, 239, 57 /*just magic number*/];
    nix::unistd::write(return_allowed_w, &wake_buf).expect("protocol failure");
    libc::exit(0);
}
