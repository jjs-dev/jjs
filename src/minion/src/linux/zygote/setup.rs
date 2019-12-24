use crate::{
    linux::{
        jail_common::{self, get_path_for_subsystem, JailOptions},
        util::{err_exit, Handle, IpcSocketExt, Pid, StraceLogger, Uid},
        zygote::{
            WM_CLASS_PID_MAP_CREATED, WM_CLASS_PID_MAP_READY_FOR_SETUP, WM_CLASS_SETUP_FINISHED,
        },
    },
    DesiredAccess, PathExpositionOptions,
};
use std::{
    collections::hash_map::DefaultHasher, ffi::CString, fs, hash::Hasher, io,
    os::unix::ffi::OsStrExt, path::Path, ptr, time,
};
use tiny_nix_ipc::Socket;

pub struct SetupData {
    pub cgroups: Vec<Handle>,
}

unsafe fn configure_dir(dir_path: &Path, uid: Uid) {
    let mode = libc::S_IRUSR
        | libc::S_IWUSR
        | libc::S_IXUSR
        | libc::S_IRGRP
        | libc::S_IWGRP
        | libc::S_IXGRP
        | libc::S_IROTH
        | libc::S_IWOTH
        | libc::S_IXOTH;
    let path = CString::new(dir_path.as_os_str().as_bytes()).unwrap();
    if libc::chmod(path.clone().as_ptr(), mode) == -1 {
        err_exit("chmod");
    }

    if libc::chown(path.as_ptr(), uid, uid) == -1 {
        err_exit("chown");
    }
}

fn expose_dir(
    jail_root: &Path,
    system_path: &Path,
    alias_path: &Path,
    access: DesiredAccess,
    uid: Uid,
) {
    let bind_target = jail_root.join(alias_path);
    fs::create_dir_all(&bind_target).unwrap();
    if fs::metadata(&system_path).unwrap().is_file() {
        fs::remove_dir(&bind_target).unwrap();
        fs::write(&bind_target, &"").unwrap();
    }
    let orig_bind_target = bind_target.clone();
    let bind_target = CString::new(bind_target.as_os_str().as_bytes()).unwrap();
    let bind_src = CString::new(system_path.as_os_str().as_bytes()).unwrap();
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
                bind_target.as_ptr(),
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

pub(crate) fn expose_dirs(expose: &[PathExpositionOptions], jail_root: &Path, uid: Uid) {
    //mount --bind
    for x in expose {
        expose_dir(jail_root, &x.src, &x.dest, x.access.clone(), uid)
    }
}

fn sigterm_handler_inner() -> ! {
    unsafe {
        libc::exit(9);
    }
}

extern "C" fn sigterm_handler(_signal: i32) {
    sigterm_handler_inner();
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
    // configure cpuacct subsystem
    let cpuacct_cgroup_path = get_path_for_subsystem("cpuacct", &jail_id);
    fs::create_dir_all(&cpuacct_cgroup_path).unwrap();

    // configure pids subsystem
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

    // we return handles to tasksfiles for main cgroups
    // so, though zygote itself and children are in chroot, and cannot access cgroupfs, they will be able to add themselves to cgroups
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
    if libc::unshare(libc::CLONE_NEWNET | libc::CLONE_NEWUSER) == -1 {
        err_exit("unshare")
    }
}

unsafe fn setup_chroot(jail_options: &JailOptions) {
    let path = jail_options.isolation_root.clone();
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    libc::open(path.as_ptr(), 0);
    if libc::chroot(path.as_ptr()) == -1 {
        err_exit("chroot");
    }
    let path_root = CString::new("/").unwrap();
    if libc::chdir(path_root.as_ptr()) == -1 {
        err_exit("chdir");
    }
}

unsafe fn setup_procfs(jail_options: &JailOptions) {
    let procfs_path = jail_options.isolation_root.join(Path::new("proc"));
    match fs::create_dir(&procfs_path) {
        Ok(_) => (),
        Err(e) => match e.kind() {
            io::ErrorKind::AlreadyExists => (),
            _ => Err(e).unwrap(),
        },
    }
    let proc = CString::new("proc").unwrap();
    let targ = CString::new(procfs_path.as_os_str().as_bytes()).unwrap();
    let mret = libc::mount(proc.as_ptr(), targ.as_ptr(), proc.as_ptr(), 0, ptr::null());
    if -1 == mret {
        err_exit("mount")
    }
}

unsafe fn setup_uid_mapping(sock: &mut Socket) -> crate::Result<()> {
    sock.wake(WM_CLASS_PID_MAP_READY_FOR_SETUP)?;
    sock.lock(WM_CLASS_PID_MAP_CREATED)?;
    if libc::setuid(0) == -1 {
        err_exit("setuid");
    }
    Ok(())
}

unsafe fn setup_time_watch(jail_options: &JailOptions) -> crate::Result<()> {
    let cpu_tl = jail_options.cpu_time_limit.as_nanos() as u64;
    let real_tl = jail_options.real_time_limit.as_nanos() as u64;
    observe_time(
        &jail_options.jail_id,
        cpu_tl,
        real_tl,
        jail_options.watchdog_chan,
    )
}

unsafe fn setup_expositions(options: &JailOptions, uid: Uid) {
    expose_dirs(&options.exposed_paths, &options.isolation_root, uid);
}

/// Derives user_ids (in range 1_000_000 to 3_000_000) from jail_id in deterministic way
pub fn derive_user_ids(jail_id: &str) -> Uid {
    let jail_id = jail_id.as_bytes();
    let mut hasher = DefaultHasher::new();
    hasher.write(jail_id);
    (hasher.finish() % 2_000_000 + 1_000_000) as Uid
}

fn setup_panic_hook() {
    use std::io::Write;
    std::panic::set_hook(Box::new(|info| {
        let mut logger = StraceLogger::new();
        write!(logger, "PANIC: {}", info).ok();
        let bt = backtrace::Backtrace::new();
        write!(logger, "{:?}", &bt).ok();
        // Now write same to stdout
        unsafe {
            logger.set_fd(0);
        }
        write!(logger, "PANIC: {}", info).ok();
        write!(logger, "{:?}", &bt).ok();
        unsafe {
            libc::abort();
        }
    }));
}

pub(crate) unsafe fn setup(
    jail_params: &JailOptions,
    sock: &mut Socket,
) -> crate::Result<SetupData> {
    setup_panic_hook();
    let uid = derive_user_ids(&jail_params.jail_id);
    configure_dir(&jail_params.isolation_root, uid);
    setup_sighandler();
    setup_expositions(&jail_params, uid);
    setup_procfs(&jail_params);
    let handles = setup_cgroups(&jail_params);
    // It's important cpu watcher will be outside of user namespace.
    setup_time_watch(&jail_params)?;
    setup_namespaces(&jail_params);
    setup_uid_mapping(sock)?;
    setup_chroot(&jail_params);
    sock.wake(WM_CLASS_SETUP_FINISHED)?;
    let res = SetupData { cgroups: handles };
    Ok(res)
}

/// Internal function, kills processes which used all their CPU time limit.
/// Limits are given in nanoseconds
unsafe fn cpu_time_observer(
    jail_id: &str,
    cpu_time_limit: u64,
    real_time_limit: u64,
    chan: std::os::unix::io::RawFd,
) -> ! {
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
        if was_cpu_tle {
            nix::unistd::write(chan, b"c").ok();
        } else if was_real_tle {
            nix::unistd::write(chan, b"r").ok();
        }
        // since we are inside pid ns, we can refer to zygote as pid1.
        jail_common::dominion_kill_all(1 as Pid).unwrap();
        // we will be killed by kernel too
    }
}

unsafe fn observe_time(
    jail_id: &str,
    cpu_time_limit: u64,
    real_time_limit: u64,
    chan: crate::linux::util::Handle,
) -> crate::Result<()> {
    let fret = libc::fork();
    if fret == -1 {
        crate::errors::System {
            code: nix::errno::errno(),
        }
        .fail()?;
    }
    if fret == 0 {
        cpu_time_observer(jail_id, cpu_time_limit, real_time_limit, chan)
    } else {
        Ok(())
    }
}
