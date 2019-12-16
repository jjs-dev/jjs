use crate::{
    linux::{
        jail_common,
        util::{err_exit, ExitCode, Handle, IpcSocketExt, Pid},
        zygote,
    },
    Dominion, DominionOptions,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{CString, OsStr, OsString},
    fmt::{self, Debug},
    fs,
    os::unix::io::AsRawFd,
    path::{Path, PathBuf},
    time::Duration,
};
use tiny_nix_ipc::Socket;

#[repr(C)]
pub struct LinuxDominion {
    id: String,
    options: DominionOptions,
    zygote_sock: Socket,
    util_cgroup_path: OsString,
    zygote_pid: Pid,
}

#[derive(Debug)]
struct LinuxDominionDebugHelper<'a> {
    id: &'a str,
    options: &'a DominionOptions,
    zygote_sock: Handle,
    util_cgroup_path: &'a OsStr,
    zygote_pid: Pid,
}

impl Debug for LinuxDominion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let h = LinuxDominionDebugHelper {
            id: &self.id,
            options: &self.options,
            zygote_sock: self.zygote_sock.as_raw_fd(),
            util_cgroup_path: &self.util_cgroup_path,
            zygote_pid: self.zygote_pid,
        };

        h.fmt(f)
    }
}

impl Dominion for LinuxDominion {
    fn id(&self) -> String {
        self.id.clone()
    }
}

/// Mount options.
/// * Readonly: jailed process can read & execute, but not write to
/// * Full: jailed process can read & write & execute
///
/// Anyway, SUID-bit will be disabled.
///
/// Warning: this type is __unstable__ (i.e. not covered by SemVer) and __non-portable__
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DesiredAccess {
    Readonly,
    Full,
}

pub(crate) struct ExtendedJobQuery {
    pub(crate) job_query: jail_common::JobQuery,
    pub(crate) stdin: Handle,
    pub(crate) stdout: Handle,
    pub(crate) stderr: Handle,
}

impl LinuxDominion {
    pub(crate) unsafe fn create(options: DominionOptions) -> crate::Result<LinuxDominion> {
        let jail_id = jail_common::gen_jail_id();
        let jail_options = jail_common::JailOptions {
            max_alive_process_count: options.max_alive_process_count,
            memory_limit: options.memory_limit,
            time_limit: options.time_limit,
            wall_time_limit: Duration::from_nanos(options.time_limit.as_nanos() as u64 * 3),
            isolation_root: options.isolation_root.clone(),
            exposed_paths: options.exposed_paths.clone(),
            jail_id: jail_id.clone(),
        };
        let startup_info = zygote::start_zygote(jail_options)?;

        Ok(LinuxDominion {
            id: jail_id,
            options,
            zygote_sock: startup_info.socket,
            util_cgroup_path: startup_info.wrapper_cgroup_path,
            zygote_pid: startup_info.zygote_pid,
        })
    }

    pub(crate) unsafe fn exit(&self) -> crate::Result<()> {
        jail_common::dominion_kill_all(self.zygote_pid)?;
        Ok(())
    }

    pub(crate) unsafe fn spawn_job(
        &mut self,
        query: ExtendedJobQuery,
    ) -> Option<jail_common::JobStartupInfo> {
        let q = jail_common::Query::Spawn(query.job_query.clone());

        // note that we ignore errors, because zygote can be already killed for some reason
        self.zygote_sock.send(&q).ok();

        let fds = [query.stdin, query.stdout, query.stderr];
        let empty: u64 = 0xDEAD_F00D_B17B_00B5;
        self.zygote_sock.send_struct(&empty, Some(&fds)).ok();
        self.zygote_sock.recv().ok()
    }

    pub(crate) unsafe fn poll_job(&mut self, pid: Pid, timeout: Duration) -> Option<ExitCode> {
        let q = jail_common::Query::Poll(jail_common::PollQuery { pid, timeout });

        self.zygote_sock.send(&q).ok();
        let res = match self.zygote_sock.recv::<Option<i32>>() {
            Ok(x) => x,
            Err(_) => return None,
        };
        res.map(Into::into)
    }
}

impl Drop for LinuxDominion {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        use std::os::unix::ffi::OsStrExt;
        // kill all processes
        unsafe { self.exit() };
        // remove cgroups
        for subsys in &["pids", "memory", "cpuacct"] {
            fs::remove_dir(jail_common::get_path_for_subsystem(subsys, &self.id));
        }

        let do_umount = |inner_path: &Path| {
            let mount_path = self.options.isolation_root.join(inner_path);
            let mount_path = CString::new(mount_path.as_os_str().as_bytes()).unwrap();
            unsafe {
                if libc::umount2(mount_path.as_ptr(), libc::MNT_DETACH) == -1 {
                    err_exit("umount2");
                }
            }
        };

        do_umount(Path::new("proc"));
        fs::remove_dir(&self.options.isolation_root.join(PathBuf::from("proc")));

        for x in &self.options.exposed_paths {
            do_umount(&x.dest);
        }
    }
}
