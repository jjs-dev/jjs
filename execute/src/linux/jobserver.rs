//! this module implements a JobServer.
//! JobServer is a long-running process in dominion.
//! JobServer accepts queries for spawning child process

use crate::{
    linux::{
        dominion::{DesiredAccess, LinuxDominion},
        jail_common::{self, gen_jail_id, get_path_for_subsystem, JailOptions},
        pipe::setup_pipe,
        util::{
            allocate_memory, duplicate_string, err_exit, Handle, HandleParcel, Pid, Sock, Uid,
            WaitMessage,
        },
    },
    DominionOptions, PathExpositionOptions,
};
use libc::{c_char, c_void};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    ffi::CString,
    fs::{self, OpenOptions},
    hash::Hasher,
    io::{self, Read, Write},
    mem, ptr,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct JobQuery {
    image_path: String,
    argv: Vec<String>,
    environment: BTreeMap<String, String>,
}

struct Stdio {
    stdin: Handle,
    stdout: Handle,
    stderr: Handle,
}

impl Stdio {
    pub unsafe fn recv(sock: &mut Sock) -> Stdio {
        unsafe fn get_handle(sock: &mut Sock) -> Handle {
            let hp: HandleParcel = sock.receive().unwrap();
            hp.into_inner()
        }
        let stdin = get_handle(sock);
        let stdout = get_handle(sock);
        let stderr = get_handle(sock);
        Stdio {
            stdin,
            stdout,
            stderr,
        }
    }
}

struct JobOptions {
    exe: String,
    argv: Vec<String>,
    env: Vec<String>,
    stdio: Stdio,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Query {
    Exit,
    Spawn(JobQuery),
}

pub(crate) struct JobServerOptions {
    jail_options: JailOptions,
    sock: Sock,
}

struct DoExecArg {
    //in
    path: String,
    arguments: Vec<String>,
    environment: Vec<String>,
    stdio: Stdio,
    sock: Sock,
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

fn expose_dir(jail_root: &str, system_path: &str, alias_path: &str, access: DesiredAccess) {
    let bind_target = get_mount_target(jail_root, alias_path);
    fs::create_dir_all(&bind_target); //TODO check error
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
        if !access.w {
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

pub(crate) fn expose_dirs(expose: Vec<PathExpositionOptions>, jail_root: &str) {
    //mount --bind
    for x in expose {
        expose_dir(
            jail_root,
            &x.src,
            &x.dest,
            DesiredAccess {
                r: x.allow_read,
                w: x.allow_write,
                x: x.allow_execute,
            },
        )
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

#[allow(unreachable_code)]
extern "C" fn do_exec(arg: *mut c_void) -> i32 {
    use std::iter::FromIterator;
    unsafe {
        let arg = &mut *(arg as *mut DoExecArg);
        let path = duplicate_string(&arg.path);

        let mut argv_with_path = vec![arg.path.clone()];
        argv_with_path.append(&mut (arg.arguments.clone()));

        //duplicate argv
        let argv = duplicate_string_list(&argv_with_path);

        //duplicate envp
        let environ = arg.environment.clone();
        let envp = duplicate_string_list(&environ);

        //now we need mark all FDs as CLOEXEC for not to expose them to sandboxed process
        let fds_to_keep = vec![arg.stdio.stdin, arg.stdio.stdout, arg.stdio.stderr];
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
            let permission: WaitMessage = arg.sock.receive().unwrap();
            permission.check(WAIT_MESSAGE_CLASS_EXECVE_PERMITTED);
        }

        //cleanup (empty)

        //dup2 as late as possible for all panics to write to normal stdio instead of pipes
        libc::dup2(arg.stdio.stdin, libc::STDIN_FILENO);
        libc::dup2(arg.stdio.stdout, libc::STDOUT_FILENO);
        libc::dup2(arg.stdio.stderr, libc::STDERR_FILENO);

        //we close these FDs because they weren't affected by FD_CLOEXEC
        libc::close(arg.stdio.stdin);
        libc::close(arg.stdio.stdout);
        libc::close(arg.stdio.stderr);

        libc::execve(
            path,
            argv as *const *const c_char,
            envp as *const *const c_char,
        );
        let err_code = errno::errno().0;
        if err_code == libc::ENOENT {
            eprintln!("FATAL ERROR: executable was not found");
            libc::exit(108)
        } else {
            //execve doesn't return on success
            err_exit("execve");
        }
    }
}

unsafe fn spawn_job(options: JobOptions) {
    let (sock, child_sock) = Sock::make_pair();
    //will be passed to child process
    let mut dea = DoExecArg {
        path: options.exe,
        arguments: options.argv,
        environment: options.env,
        stdio: options.stdio,
        sock: child_sock,
    };

    let dea_ptr = &mut dea as *mut DoExecArg;

    let mut child_pid: Pid = 0;

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

    //now we can allow child to execve()
    sock.send(&WaitMessage::with_class(
        WAIT_MESSAGE_CLASS_EXECVE_PERMITTED,
    ))
    .unwrap();
}

const WAIT_CODE_DOMINION_CREATED: u16 = 1;
const WAIT_CODE_PID_MAP_READY_FOR_SETUP: u16 = 2;
const WAIT_CODE_PID_MAP_CREATED: u16 = 3;

unsafe fn setup_cgroups(jail_options: &JailOptions, jail_id: &str) {
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
}

unsafe fn setup_namespaces(jail_options: &JailOptions) {
    if libc::unshare(libc::CLONE_NEWNET | libc::CLONE_NEWUSER | libc::CLONE_NEWPID) == -1 {
        err_exit("unshare")
    }
}

unsafe fn setup_chroot(jail_options: &JailOptions) {
    let path = jail_options.isolation_root.clone();
    let path = CString::new(path.as_str()).unwrap();
    if libc::chroot(path.as_ptr()) == -1 {
        err_exit("chroot");
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

unsafe fn setup_uid_mapping(jail_options: &JailOptions, sock: &mut Sock) {
    sock.send(&WaitMessage::with_class(WAIT_CODE_PID_MAP_READY_FOR_SETUP))
        .unwrap();
    let resp = sock.receive::<WaitMessage>().unwrap();
    resp.check(WAIT_CODE_PID_MAP_CREATED).unwrap();

    sock.send(&WaitMessage::with_class(WAIT_CODE_DOMINION_CREATED))
        .unwrap();
}

struct UserIdInfo {
    privileged: Uid,
    restricted: Uid,
}

///derives user_ids (in range 1_000_000 to 2_000_000) from jail_id in determined way
fn derive_user_ids(jail_id: &str) -> UserIdInfo {
    let jail_id = jail_id.as_bytes();
    let mut hasher = DefaultHasher::new();
    hasher.write(jail_id);
    let privileged = (hasher.finish() % 2_000_000 + 1_000_000) as Uid;
    hasher.write(jail_id);
    let restricted = (hasher.finish() % 2_000_000 + 1_000_000) as Uid;
    UserIdInfo {
        privileged,
        restricted,
    }
}

fn setup(jail_params: JailOptions) {}

fn fill_pid_gid_map_for_child(sock: &mut Sock, child_pid: i32, mapped_uid: i32) {
    unsafe {}
}

unsafe fn add_to_subsys(pid: Pid, subsys: &str, jail_id: &str) {
    let cgroup_path = get_path_for_subsystem(subsys, jail_id);
    let tasks_file_path = format!("{}/tasks", cgroup_path);
    let mut f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(tasks_file_path)
        .unwrap();
    write!(f, "{}", pid).unwrap();
}

unsafe fn add_process(pid: Pid, jail_id: &str) {
    for subsys in &["pids", "memory"] {
        add_to_subsys(pid, subsys, jail_id);
    }
}

unsafe fn jobserver_entry(mut arg: JobServerOptions) -> i32 {
    setup(arg.jail_options.clone());

    loop {
        let query: String;
        query = arg.sock.receive().unwrap();
        let query: Query = serde_json::from_str(query.as_str()).unwrap();
        let options = match query {
            Query::Spawn(o) => o,
            Query::Exit => break,
        };
        //now we do some preprocessing
        let mut argv = vec![options.image_path.clone()];
        argv.extend_from_slice(&options.argv);

        let env: Vec<_> = options
            .environment
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let child_stdio = Stdio::recv(&mut arg.sock);

        let job_options = JobOptions {
            exe: options.image_path,
            argv,
            env,
            stdio: child_stdio,
        };

        spawn_job(job_options)
    }
    0
}

unsafe fn start_jobserver(jail_options: JailOptions) -> Sock {
    let (sock, js_sock) = Sock::make_pair();
    let jail_id = jail_common::gen_jail_id();

    let fret = libc::fork();
    if fret == -1 {
        err_exit("fork");
    }
    if fret == 0 {
        let js_arg = JobServerOptions {
            jail_options: jail_options.clone(),
            sock: js_sock,
        };
        let jobserver_ret_code = jobserver_entry(js_arg);
    }
    let child_pid = fret as Pid;
    {
        {
            let wm: WaitMessage = sock.receive().unwrap();
            wm.check(WAIT_CODE_PID_MAP_READY_FOR_SETUP);
        }
        let uid_info = derive_user_ids(&jail_id);
        //child will have uid=1 or 2 in its namespace, but some random and not-privileged in outer one
        let mapping = format!("1 {} 1\n2 {} 1", uid_info.privileged, uid_info.restricted);
        let uid_map_path = format!("/proc/{}/uid_map", child_pid);
        let gid_map_path = format!("/proc/{}/gid_map", child_pid);
        fs::write(&uid_map_path, mapping.as_str()).unwrap();
        fs::write(&gid_map_path, mapping.as_str()).unwrap();
        sock.send(&WaitMessage::with_class(WAIT_CODE_PID_MAP_CREATED))
            .unwrap();
        {
            let wm: WaitMessage = sock.receive().unwrap();
            wm.check(WAIT_CODE_DOMINION_CREATED).unwrap();
        }
    }
    sock
}
