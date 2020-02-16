use crate::{
    linux::{
        jail_common::{self, JailOptions},
        util::{err_exit, Handle, IpcSocketExt, Pid, StraceLogger, Uid},
        zygote::{
            WM_CLASS_PID_MAP_CREATED, WM_CLASS_PID_MAP_READY_FOR_SETUP, WM_CLASS_SETUP_FINISHED,
        },
    },
    DesiredAccess, PathExpositionOptions,
};
use std::{
    collections::hash_map::DefaultHasher, ffi::CString, fs, hash::Hasher, io, io::Write,
    os::unix::ffi::OsStrExt, path::Path, ptr, time,
};
use tiny_nix_ipc::Socket;

pub(in crate::linux) struct SetupData {
    pub(in crate::linux) cgroups: super::cgroup::Group,
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

extern "C" fn exit_sighandler(_code: i32) {
    unsafe {
        libc::exit(1);
    }
}

unsafe fn setup_sighandler() {
    use nix::sys::signal;
    for &death in &[
        signal::Signal::SIGABRT,
        signal::Signal::SIGINT,
        signal::Signal::SIGSEGV,
    ] {
        let handler = signal::SigHandler::SigDfl;
        let action =
            signal::SigAction::new(handler, signal::SaFlags::empty(), signal::SigSet::empty());

        signal::sigaction(death, &action).expect("Couldn't setup sighandler");
    }
    {
        let sigterm_handler = signal::SigHandler::Handler(exit_sighandler);
        let action = signal::SigAction::new(
            sigterm_handler,
            signal::SaFlags::empty(),
            signal::SigSet::empty(),
        );
        signal::sigaction(signal::Signal::SIGTERM, &action)
            .expect("Failed to setup SIGTERM handler");
    }
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
    std::panic::set_hook(Box::new(|info| {
        let mut logger = StraceLogger::new();
        write!(logger, "PANIC: {}", info).ok();
        let bt = backtrace::Backtrace::new();
        write!(logger, "{:?}", &bt).ok();
        // Now write same to stdout
        unsafe {
            logger.set_fd(2);
        }
        write!(logger, "PANIC: {}", info).ok();
        write!(logger, "{:?}", &bt).ok();
        write!(logger, "Exiting").ok();
        unsafe {
            libc::exit(libc::EXIT_FAILURE);
        }
    }));
}

pub(in crate::linux) unsafe fn setup(
    jail_params: &JailOptions,
    sock: &mut Socket,
) -> crate::Result<SetupData> {
    setup_panic_hook();
    setup_sighandler();
    let uid = derive_user_ids(&jail_params.jail_id);
    configure_dir(&jail_params.isolation_root, uid);
    setup_expositions(&jail_params, uid);
    setup_procfs(&jail_params);
    let handles = super::cgroup::setup_cgroups(&jail_params);
    // It's important cpu watcher will be outside of user namespace.
    setup_time_watch(&jail_params)?;
    setup_namespaces(&jail_params);
    setup_uid_mapping(sock)?;
    setup_chroot(&jail_params);
    sock.wake(WM_CLASS_SETUP_FINISHED)?;
    let mut logger = crate::linux::util::StraceLogger::new();
    writeln!(logger, "dominion {}: setup done", &jail_params.jail_id).unwrap();
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
    let mut logger = crate::linux::util::StraceLogger::new();
    writeln!(logger, "dominion {}: cpu time watcher", jail_id).unwrap();
    let start = time::Instant::now();
    loop {
        libc::sleep(1);

        let elapsed = time::Instant::now().duration_since(start);
        let elapsed = elapsed.as_nanos();
        let current_usage = super::cgroup::get_cpu_usage(jail_id);
        let was_cpu_tle = current_usage > cpu_time_limit;
        let was_real_tle = elapsed as u64 > real_time_limit;
        let ok = !was_cpu_tle && !was_real_tle;
        if ok {
            continue;
        }
        if was_cpu_tle {
            eprintln!("CPU time limit exceeded");
            nix::unistd::write(chan, b"c").ok();
        } else if was_real_tle {
            eprintln!(
                "Real time limit exceeded: limit {}, used {}",
                real_time_limit, elapsed
            );
            nix::unistd::write(chan, b"r").ok();
        }
        // since we are inside pid ns, we can refer to zygote as pid1.
        let err = jail_common::dominion_kill_all(1 as Pid, None);
        if let Err(err) = err {
            eprintln!("failed to kill dominion {:?}", err);
        }
        // we will be killed by kernel too
    }
}

unsafe fn observe_time(
    jail_id: &str,
    cpu_time_limit: u64,
    real_time_limit: u64,
    chan: Handle,
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
