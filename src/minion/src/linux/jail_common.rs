use crate::{linux::util::Pid, PathExpositionOptions};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ffi::OsString, path::PathBuf, time::Duration};
use tiny_nix_ipc::Socket;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct JailOptions {
    pub(crate) max_alive_process_count: u32,
    pub(crate) memory_limit: u64,
    /// Specifies total CPU time for whole dominion.
    pub(crate) time_limit: Duration,
    /// Specifies wall-closk time limit for whole dominion.
    /// Possible value: time_limit * 3.
    pub(crate) wall_time_limit: Duration,
    pub(crate) isolation_root: PathBuf,
    pub(crate) exposed_paths: Vec<PathExpositionOptions>,
    pub(crate) jail_id: String,
}

pub(crate) fn get_path_for_subsystem(subsys_name: &str, cgroup_id: &str) -> String {
    format!("/sys/fs/cgroup/{}/jjs/g-{}", subsys_name, cgroup_id)
}

const ID_CHARS: &[u8] = b"qwertyuiopasdfghjklzxcvbnm1234567890";
const ID_SIZE: usize = 8;

pub(crate) fn gen_jail_id() -> String {
    let mut gen = rand::thread_rng();
    let mut out = Vec::new();
    for _i in 0..ID_SIZE {
        let ch = *(ID_CHARS.choose(&mut gen).unwrap());
        out.push(ch);
    }
    String::from_utf8_lossy(&out[..]).to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct JobQuery {
    pub(crate) image_path: PathBuf,
    pub(crate) argv: Vec<OsString>,
    pub(crate) environment: HashMap<String, OsString>,
    pub(crate) pwd: PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct PollQuery {
    pub(crate) pid: Pid,
    pub(crate) timeout: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct JobStartupInfo {
    pub(crate) pid: Pid,
}

pub(crate) struct ZygoteStartupInfo {
    pub(crate) socket: Socket,
    pub(crate) zygote_pid: Pid,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum Query {
    Exit,
    Spawn(JobQuery),
    Poll(PollQuery),
}

pub(crate) fn dominion_kill_all(zygote_pid: Pid) -> crate::Result<()> {
    // We will send SIGTERM to zygote, and
    // kernel will kill all other processes by itself.
    unsafe {
        libc::kill(zygote_pid, libc::SIGTERM);
    }
    Ok(())
}
