use crate::{
    linux::{
        jail_common, jobserver,
        util::{self, allocate_heap_variable, err_exit, ExitCode, Handle, IpcSocketExt, Pid},
    },
    Dominion, DominionOptions,
};
use failure::ResultExt;
use field_offset::offset_of;
use std::{
    ffi::CString,
    fmt::{self, Debug},
    fs,
    io::Write,
    os::unix::io::AsRawFd,
    ptr,
    time::Duration,
};
use tiny_nix_ipc::Socket;

#[repr(C)]
pub struct LinuxDominion {
    id: String,
    options: DominionOptions,
    jobserver_sock: Socket,
}

#[derive(Debug)]
struct LinuxDominionDebugHelper {
    id: String,
    options: DominionOptions,
    jobserver_sock: Handle,
}

impl Debug for LinuxDominion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let h = LinuxDominionDebugHelper {
            id: self.id.clone(),
            options: self.options.clone(),
            jobserver_sock: self.jobserver_sock.as_raw_fd(),
        };

        h.fmt(f)
    }
}

impl Dominion for LinuxDominion {}

//TODO extract to crate
#[allow(dead_code)]
fn dev_log(s: &str) {
    let c = CString::new(s).unwrap();
    unsafe {
        libc::write(-1, c.as_ptr() as *const libc::c_void, s.len());
    }
}

/// Mounting options.
/// * Readonly: jailed process can read & execute, but not write to
/// * Full: jailed process can read & write & execute
///
/// Anyway, SUID-bit will be disabled.
///
/// Warning: this type is __unstable__ (i.e. not covered by SemVer) and __non-portable__
///
///
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
    pub(crate) unsafe fn create(options: DominionOptions) -> LinuxDominion {
        let jail_id = jail_common::gen_jail_id();
        let jail_options = jail_common::JailOptions {
            allow_network: options.allow_network,
            allow_file_io: options.allow_file_io,
            max_alive_process_count: options.max_alive_process_count,
            memory_limit: options.memory_limit,
            time_limit: options.time_limit,
            isolation_root: options.isolation_root.to_str().unwrap().to_string(),
            exposed_paths: options.exposed_paths.clone(),
            jail_id: jail_id.clone(),
        };
        let sock = jobserver::start_jobserver(jail_options);

        LinuxDominion {
            id: jail_id.clone(),
            options: options.clone(),
            jobserver_sock: sock,
        }
    }

    pub(crate) fn exit(&mut self) -> crate::Result<()> {
        //we just need to kill all processes in pids (e.g.) cgroup
        let pids_cgroup_path = jail_common::get_path_for_subsystem("pids", &self.id);

        //step 1: disallow forking
        let pids_max_file_path = format!("{}/pids.max", &pids_cgroup_path);
        fs::write(pids_max_file_path, "0").context(crate::ErrorKind::Io)?;

        let cgroup_members_path = format!("{}/tasks", &pids_cgroup_path);
        let cgroup_members =
            fs::read_to_string(cgroup_members_path).context(crate::ErrorKind::Io)?;

        let mut cgroup_members = cgroup_members.split("\n");
        while let Some(pid) = cgroup_members.next() {
            let pid : String= pid.to_string();
            let pid = pid.trim().to_string();
            if pid.len() == 0 {
                //skip last, empty line
                continue
            }
            let pid: Pid = pid.parse().unwrap();
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }

        Ok(())
    }

    pub(crate) unsafe fn spawn_job(
        &mut self,
        query: ExtendedJobQuery,
    ) -> jail_common::JobStartupInfo {
        let mut logger = util::strace_logger();
        write!(logger, "sending queries to jobserver");
        let q = jail_common::Query::Spawn(query.job_query.clone());

        let fds = [query.stdin, query.stdout, query.stderr];
        self.jobserver_sock.send(&q).unwrap();

        let empty: u64 = 0xDEAD_F00D_B17B_00B5;
        self.jobserver_sock.send_struct(&empty, Some(&fds)).unwrap();
        write!(logger, "waiting for jobserver");
        self.jobserver_sock.recv().unwrap()
    }

    pub(crate) unsafe fn poll_job(&mut self, pid: Pid, timeout: Duration) -> Option<ExitCode> {
        let q = jail_common::Query::Poll(jail_common::PollQuery { pid, timeout });

        self.jobserver_sock.send(&q).unwrap();
        self.jobserver_sock.recv().unwrap()
    }
}

impl Drop for LinuxDominion {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        self.exit();
        //remove cgroups
        for subsys in &["pids", "memory", "cpuacct"] {
            fs::remove_dir(jail_common::get_path_for_subsystem(subsys, &self.id));
        }

        let do_umount = |inner_path: &str| {
            let mount_path = format!(
                "{}/{}",
                &self.options.isolation_root.to_str().unwrap(),
                inner_path
            );
            let mount_path = CString::new(mount_path.as_str()).unwrap();
            unsafe {
                if libc::umount2(mount_path.as_ptr(), libc::MNT_DETACH) == -1 {
                    err_exit("umount2");
                }
            }
        };

        do_umount("/proc");
        for x in &self.options.exposed_paths {
            do_umount(x.dest.as_str());
        }
    }
}
