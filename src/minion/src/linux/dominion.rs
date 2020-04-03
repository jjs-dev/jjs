use crate::{
    linux::{
        jail_common,
        pipe::setup_pipe,
        util::{err_exit, ExitCode, Handle, IpcSocketExt, Pid},
        zygote,
    },
    Dominion, DominionOptions,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Debug},
    os::unix::io::AsRawFd,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Mutex,
    },
    time::Duration,
};
use tiny_nix_ipc::Socket;

/// Bits which are reported by time watcher
#[derive(Debug)]
#[repr(C)]
struct DominionState {
    /// CPU time limit was exceeded
    was_cpu_tle: AtomicBool,
    /// Wall-clock time limit was exceeded
    was_wall_tle: AtomicBool,
}

impl DominionState {
    fn process_flag(&self, ch: u8) -> crate::Result<()> {
        match ch {
            b'c' => {
                self.was_cpu_tle.store(true, SeqCst);
            }
            b'r' => {
                self.was_wall_tle.store(true, SeqCst);
            }
            _ => return Err(crate::Error::Sandbox),
        }
        Ok(())
    }

    fn clone(&self) -> Self {
        DominionState {
            was_cpu_tle: AtomicBool::new(self.was_cpu_tle.load(SeqCst)),
            was_wall_tle: AtomicBool::new(self.was_wall_tle.load(SeqCst)),
        }
    }
}
#[repr(C)]
pub struct LinuxDominion {
    id: String,
    options: DominionOptions,
    zygote_sock: Mutex<Socket>,
    zygote_pid: Pid,
    state: DominionState,
    watchdog_chan: Handle,
}

#[derive(Debug)]
struct LinuxDominionDebugHelper<'a> {
    id: &'a str,
    options: &'a DominionOptions,
    zygote_sock: Handle,
    zygote_pid: Pid,
    state: DominionState,
    watchdog_chan: Handle,
}

impl Debug for LinuxDominion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let h = LinuxDominionDebugHelper {
            id: &self.id,
            options: &self.options,
            zygote_sock: self.zygote_sock.lock().unwrap().as_raw_fd(),
            zygote_pid: self.zygote_pid,
            watchdog_chan: self.watchdog_chan,
            state: self.state.clone(),
        };

        h.fmt(f)
    }
}

impl Dominion for LinuxDominion {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn check_cpu_tle(&self) -> crate::Result<bool> {
        self.poll_state()?;
        Ok(self.state.was_cpu_tle.load(SeqCst))
    }

    fn check_real_tle(&self) -> crate::Result<bool> {
        self.poll_state()?;
        Ok(self.state.was_wall_tle.load(SeqCst))
    }

    fn kill(&self) -> crate::Result<()> {
        jail_common::dominion_kill_all(self.zygote_pid, Some(&self.id))
            .map_err(|err| crate::Error::Io { source: err })?;
        Ok(())
    }

    fn resource_usage(&self) -> crate::Result<crate::ResourceUsageData> {
        let cpu_usage = zygote::cgroup::get_cpu_usage(&self.id);
        let memory_usage = zygote::cgroup::get_memory_usage(&self.id);
        Ok(crate::ResourceUsageData {
            memory: memory_usage,
            time: Some(cpu_usage),
        })
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
    fn poll_state(&self) -> crate::Result<()> {
        for _ in 0..5 {
            let mut buf = [0; 4];
            let num_read = nix::unistd::read(self.watchdog_chan, &mut buf).or_else(|err| {
                if let Some(errno) = err.as_errno() {
                    if errno as i32 == libc::EAGAIN {
                        return Ok(0);
                    }
                }
                Err(err)
            })?;
            if num_read == 0 {
                break;
            }
            for ch in &buf[..num_read] {
                self.state.process_flag(*ch)?;
            }
        }

        Ok(())
    }

    pub(crate) unsafe fn create(options: DominionOptions) -> crate::Result<LinuxDominion> {
        let jail_id = jail_common::gen_jail_id();
        let mut read_end = 0;
        let mut write_end = 0;
        setup_pipe(&mut read_end, &mut write_end)?;
        if -1 == libc::fcntl(read_end, libc::F_SETFL, libc::O_NONBLOCK) {
            err_exit("fcntl");
        }
        let jail_options = jail_common::JailOptions {
            max_alive_process_count: options.max_alive_process_count,
            memory_limit: options.memory_limit,
            cpu_time_limit: options.cpu_time_limit,
            real_time_limit: options.real_time_limit,
            isolation_root: options.isolation_root.clone(),
            exposed_paths: options.exposed_paths.clone(),
            jail_id: jail_id.clone(),
            watchdog_chan: write_end,
        };
        let startup_info = zygote::start_zygote(jail_options)?;

        Ok(LinuxDominion {
            id: jail_id,
            options,
            zygote_sock: Mutex::new(startup_info.socket),
            zygote_pid: startup_info.zygote_pid,
            watchdog_chan: read_end,
            state: DominionState {
                was_cpu_tle: AtomicBool::new(false),
                was_wall_tle: AtomicBool::new(false),
            },
        })
    }

    pub(crate) unsafe fn spawn_job(
        &self,
        query: ExtendedJobQuery,
    ) -> Option<jail_common::JobStartupInfo> {
        let q = jail_common::Query::Spawn(query.job_query.clone());

        // note that we ignore errors, because zygote can be already killed for some reason
        self.zygote_sock.lock().unwrap().send(&q).ok();

        let fds = [query.stdin, query.stdout, query.stderr];
        let empty: u64 = 0xDEAD_F00D_B17B_00B5;
        self.zygote_sock
            .lock()
            .unwrap()
            .send_struct(&empty, Some(&fds))
            .ok();
        self.zygote_sock.lock().unwrap().recv().ok()
    }

    pub(crate) unsafe fn poll_job(&self, pid: Pid, timeout: Option<Duration>) -> Option<ExitCode> {
        let q = jail_common::Query::Poll(jail_common::PollQuery { pid, timeout });
        self.zygote_sock.lock().unwrap().send(&q).ok();
        match self.zygote_sock.lock().unwrap().recv::<Option<i32>>() {
            Ok(x) => x,
            Err(_) => None,
        }
    }
}

impl Drop for LinuxDominion {
    fn drop(&mut self) {
        // Kill all processes.
        if let Err(err) = self.kill() {
            panic!("unable to kill dominion: {}", err);
        }
        // Remove cgroups.
        if std::env::var("MINION_DEBUG_KEEP_CGROUPS").is_err() {
            zygote::cgroup::drop(&self.id, &["pids", "memory", "cpuacct"]);
        }

        // Close handles
        nix::unistd::close(self.watchdog_chan).ok();
    }
}
