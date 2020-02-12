use crate::linux::{
    jail_common::{get_path_for_subsystem, JailOptions},
    util::{err_exit, Handle, Pid},
};
use std::fs;

pub(super) unsafe fn setup_cgroups(jail_options: &JailOptions) -> Vec<Handle> {
    let jail_id = &jail_options.jail_id;
    // configure cpuacct subsystem
    let cpuacct_cgroup_path = get_path_for_subsystem("cpuacct", &jail_id);
    fs::create_dir_all(&cpuacct_cgroup_path).expect("failed to create cpuacct cgroup");

    // configure pids subsystem
    let pids_cgroup_path = get_path_for_subsystem("pids", &jail_id);
    fs::create_dir_all(&pids_cgroup_path).expect("failed to create pids cgroup");

    fs::write(
        pids_cgroup_path.join("pids.max"),
        format!(
            "{}",
            jail_options.max_alive_process_count + 2 /* to account for zygote and time watcher */
        ),
    )
    .expect("failed to enable pids limit");

    //configure memory subsystem
    let mem_cgroup_path = get_path_for_subsystem("memory", &jail_id);

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
    let mut handles = ["cpuacct", "memory", "pids"]
        .iter()
        .map(|subsys_name| {
            use std::os::unix::io::IntoRawFd;
            let p = get_path_for_subsystem(subsys_name, &jail_id);
            let p = p.join("tasks");
            let h = fs::OpenOptions::new()
                .write(true)
                .open(&p)
                .unwrap_or_else(|err| panic!("Couldn't open tasks file {}: {}", p.display(), err))
                .into_raw_fd();
            libc::dup(h)
        })
        .collect::<Vec<_>>();
    let pids_cgroup_handle = handles.pop().expect("must have len == 3");
    nix::unistd::write(pids_cgroup_handle, b"1").expect("failed to join pids cgroup");
    handles
}
