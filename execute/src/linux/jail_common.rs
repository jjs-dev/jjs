use crate::{linux::util::Pid, PathExpositionOptions};
use rand::seq::SliceRandom;
use std::{collections::BTreeMap, time::Duration};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct JailOptions {
    pub(crate) allow_network: bool,
    pub(crate) allow_file_io: bool,
    pub(crate) max_alive_process_count: usize,
    pub(crate) memory_limit: usize,
    ///specifies total CPU time for all dominion
    pub(crate) time_limit: Duration,
    pub(crate) isolation_root: String,
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
    pub(crate) image_path: String,
    pub(crate) argv: Vec<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) pwd: String,
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

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum Query {
    Exit,
    Spawn(JobQuery),
    Poll(PollQuery),
}
