use crate::linux::{
    jail_common::{get_path_for_cgroup_legacy_subsystem, JailOptions},
    util::{err_exit, Handle, Pid},
};
use std::{
    fs,
    os::unix::io::IntoRawFd,
    path::PathBuf,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};
#[derive(Clone)]
enum GroupHandles {
    // For cgroups V1, we store handles of `tasks` file in each hierarchy.
    V1(Vec<Handle>),
    // For cgroups V2, we store handle of `cgroup.procs` file in cgroup dir.
    V2(Handle),
}

#[derive(Clone)]
pub(in crate::linux) struct Group {
    handles: GroupHandles,
    id: String,
}

impl Group {
    pub(super) fn join_self(&self) {
        let mut slice_iter;
        let mut once_iter;
        let it: &mut dyn std::iter::Iterator<Item = Handle> = match &self.handles {
            GroupHandles::V1(handles) => {
                slice_iter = handles.iter().copied();
                &mut slice_iter
            }
            GroupHandles::V2(handle) => {
                once_iter = std::iter::once(*handle);
                &mut once_iter
            }
        };
        let my_pid = std::process::id();
        let my_pid = format!("{}", my_pid);
        for h in it {
            nix::unistd::write(h, my_pid.as_bytes()).expect("Couldn't join cgroup");
        }
    }
}
#[derive(Eq, PartialEq)]
pub(in crate::linux) enum CgroupVersion {
    V1,
    V2,
}

const CGROUP_VERSION_1: u8 = 1;
const CGROUP_VERSION_2: u8 = 2;

fn do_detect_cgroup_version() -> u8 {
    let stat =
        nix::sys::statfs::statfs("/sys/fs/cgroup").expect("/sys/fs/cgroup is not root of cgroupfs");
    let ty = stat.filesystem_type();
    // man 2 statfs
    match ty.0 {
        0x0027_e0eb => return CGROUP_VERSION_1,
        0x6367_7270 => return CGROUP_VERSION_2,
        _ => (),
    };
    let p = std::path::Path::new("/sys/fs/cgroup");
    if p.join("cgroup.subtree_control").exists() {
        CGROUP_VERSION_2
    } else {
        CGROUP_VERSION_1
    }
}

pub(in crate::linux) fn detect_cgroup_version() -> CgroupVersion {
    static CACHE: AtomicU8 = AtomicU8::new(0);
    if CACHE.load(Ordering::Relaxed) == 0 {
        let version = do_detect_cgroup_version();
        CACHE.store(version, Ordering::Relaxed);
    }
    match CACHE.load(Ordering::Relaxed) {
        CGROUP_VERSION_1 => CgroupVersion::V1,
        CGROUP_VERSION_2 => CgroupVersion::V2,
        val => unreachable!("unexpected value in cgroup version cache: {}", val),
    }
}

fn do_get_cgroup_prefix() -> String {
    // TODO: take from config
    "/jjs".to_string()
}

pub(crate) fn get_cgroup_prefix() -> &'static PathBuf {
    static CACHE: AtomicUsize = AtomicUsize::new(0);
    if CACHE.load(Ordering::Relaxed) == 0 {
        let sub_path = do_get_cgroup_prefix();
        let mut buf: PathBuf = "/sys/fs/cgroup".into();
        assert!(sub_path.starts_with('/'));
        buf.push(&sub_path[1..]);
        let ptr = Box::leak(Box::new(buf)) as &PathBuf;
        CACHE.store(ptr as *const PathBuf as usize, Ordering::Relaxed);
        ptr
    } else {
        unsafe { &*(CACHE.load(Ordering::Relaxed) as *const PathBuf) }
    }
}

pub(crate) fn get_path_for_cgroup_unified(cgroup_id: &str) -> PathBuf {
    get_cgroup_prefix()
        .join("jjs")
        .join(format!("sandbox.{}", cgroup_id))
}

unsafe fn setup_chroups_legacy(jail_options: &JailOptions) -> Vec<Handle> {
    let jail_id = &jail_options.jail_id;
    // configure cpuacct subsystem
    let cpuacct_cgroup_path = get_path_for_cgroup_legacy_subsystem("cpuacct", &jail_id);
    fs::create_dir_all(&cpuacct_cgroup_path).expect("failed to create cpuacct cgroup");

    // configure pids subsystem
    let pids_cgroup_path = get_path_for_cgroup_legacy_subsystem("pids", &jail_id);
    fs::create_dir_all(&pids_cgroup_path).expect("failed to create pids cgroup");

    fs::write(
        pids_cgroup_path.join("pids.max"),
        format!("{}", jail_options.max_alive_process_count),
    )
    .expect("failed to enable pids limit");

    //configure memory subsystem
    let mem_cgroup_path = get_path_for_cgroup_legacy_subsystem("memory", &jail_id);

    fs::create_dir_all(&mem_cgroup_path).expect("failed to create memory cgroup");
    fs::write(mem_cgroup_path.join("memory.swappiness"), "0").expect("failed to disallow swapping");

    fs::write(
        mem_cgroup_path.join("memory.limit_in_bytes"),
        format!("{}", jail_options.memory_limit),
    )
    .expect("failed to enable memory limiy");

    let my_pid: Pid = libc::getpid();
    if my_pid == -1 {
        err_exit("getpid");
    }

    // we return handles to tasksfiles for main cgroups
    // so, though zygote itself and children are in chroot, and cannot access cgroupfs, they will be able to add themselves to cgroups
    ["cpuacct", "memory", "pids"]
        .iter()
        .map(|subsys_name| {
            let p = get_path_for_cgroup_legacy_subsystem(subsys_name, &jail_id);
            let p = p.join("tasks");
            let h = fs::OpenOptions::new()
                .write(true)
                .open(&p)
                .unwrap_or_else(|err| panic!("Couldn't open tasks file {}: {}", p.display(), err))
                .into_raw_fd();
            libc::dup(h)
        })
        .collect::<Vec<_>>()
}

unsafe fn setup_cgroups_v2(jail_options: &JailOptions) -> Handle {
    let jail_id = &jail_options.jail_id;
    let cgroup_path = get_path_for_cgroup_unified(jail_id);
    fs::create_dir_all(&cgroup_path).expect("failed to create cgroup");

    fs::write(
        cgroup_path.parent().unwrap().join("cgroup.subtree_control"),
        "+pids +cpu +memory",
    )
    .ok();

    fs::write(
        cgroup_path.join("pids.max"),
        format!("{}", jail_options.max_alive_process_count),
    )
    .expect("failed to set pids.max limit");

    fs::write(
        cgroup_path.join("memory.max"),
        format!("{}", jail_options.memory_limit),
    )
    .expect("failed to set memory limit");

    let tasks_file_path = cgroup_path.join("cgroup.procs");
    let h = fs::OpenOptions::new()
        .write(true)
        .open(&tasks_file_path)
        .unwrap_or_else(|err| {
            panic!(
                "Failed to open tasks file {}: {}",
                tasks_file_path.display(),
                err
            )
        });
    libc::dup(h.into_raw_fd())
}

pub(super) unsafe fn setup_cgroups(jail_options: &JailOptions) -> Group {
    let handles = match detect_cgroup_version() {
        CgroupVersion::V1 => GroupHandles::V1(setup_chroups_legacy(jail_options)),
        CgroupVersion::V2 => GroupHandles::V2(setup_cgroups_v2(jail_options)),
    };
    Group {
        handles,
        id: jail_options.jail_id.clone(),
    }
}

pub(in crate::linux) fn get_cpu_usage(jail_id: &str) -> u64 {
    match detect_cgroup_version() {
        CgroupVersion::V1 => {
            let current_usage_file = get_path_for_cgroup_legacy_subsystem("cpuacct", jail_id);
            let current_usage_file = current_usage_file.join("cpuacct.usage");
            fs::read_to_string(current_usage_file)
                .expect("Couldn't load cpu usage")
                .trim()
                .parse::<u64>()
                .unwrap()
        }
        CgroupVersion::V2 => {
            let mut current_usage_file = get_path_for_cgroup_unified(jail_id);
            current_usage_file.push("cpu.stat");
            let stat_data =
                fs::read_to_string(current_usage_file).expect("failed to read cpu.stat");
            let mut val = 0;
            for line in stat_data.lines() {
                if line.starts_with("usage_usec") {
                    let usage = line
                        .trim_start_matches("usage_usec ")
                        .trim_end_matches('\n');
                    val = usage.parse().unwrap();
                }
            }
            // multiply by 1000 to convert from microseconds to nanoseconds
            val * 1000
        }
    }
}

pub(in crate::linux) fn get_memory_usage(jail_id: &str) -> Option<u64> {
    match detect_cgroup_version() {
        // memory cgroup v2 does not provide way to get peak memory usage.
        // `memory.current` contains only current usage.
        CgroupVersion::V2 => None,
        CgroupVersion::V1 => {
            let mut current_usage_file = get_path_for_cgroup_legacy_subsystem("memory", jail_id);
            current_usage_file.push("memory.max_usage_in_bytes");
            let usage = fs::read_to_string(current_usage_file)
                .expect("cannot read memory usage")
                .trim()
                .parse::<u64>()
                .unwrap();
            Some(usage)
        }
    }
}

pub(in crate::linux) fn drop(jail_id: &str, legacy_subsystems: &[&str]) {
    match detect_cgroup_version() {
        CgroupVersion::V1 => {
            for subsys in legacy_subsystems {
                fs::remove_dir(get_path_for_cgroup_legacy_subsystem(subsys, jail_id)).ok();
            }
        }
        CgroupVersion::V2 => {
            fs::remove_dir(get_path_for_cgroup_unified(jail_id)).ok();
        }
    }
}

pub(in crate::linux) fn get_cgroup_tasks_file_path(jail_id: &str) -> PathBuf {
    match detect_cgroup_version() {
        CgroupVersion::V1 => get_path_for_cgroup_legacy_subsystem("pids", jail_id).join("tasks"),
        CgroupVersion::V2 => get_path_for_cgroup_unified(jail_id).join("cgroup.procs"),
    }
}
