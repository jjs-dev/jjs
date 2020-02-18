use crate::linux::zygote::cgroup::{detect_cgroup_version, CgroupVersion};
use std::path::PathBuf;

fn get_current_cgroup_path() -> Vec<String> {
    let group = procfs::process::Process::myself()
        .expect("failed to load myself data from procfs")
        .cgroups()
        .expect("/proc/self/cgroups is unreadable");
    let first = group
        .into_iter()
        .next()
        .expect("/proc/self/cgroups is empty");
    assert!(!first.controllers.is_empty());
    assert_eq!(first.hierarchy, 0);
    let pathname = first.pathname;
    pathname.split('/').map(ToString::to_string).collect()
}

fn get_sandbox_chgroup_path() -> Vec<String> {
    vec!["jjs".to_string()]
}

fn check_has_cgroup_access(cgroup: &[String]) -> Result<(), String> {
    let mut cgroup_path = PathBuf::from("/sys/fs/cgroup");
    for item in cgroup {
        cgroup_path.push(item);
    }
    let temp_dir_name = format!(
        "jjs-minion-acc-ck-{}",
        rand::random::<[char; 6]>().iter().collect::<String>()
    );
    let temp_dir_path = cgroup_path.join(temp_dir_name);
    if let Err(err) = std::fs::create_dir(&temp_dir_path) {
        return Err(format!("unable to create groups: {}", err));
    }
    if let Err(err) = std::fs::remove_dir(temp_dir_path) {
        return Err(format!("failed to cleanup after check: {}", err));
    }
    let subtree_controllers = cgroup_path.join("cgroup.subtree_control");
    let controllers = match std::fs::read_to_string(subtree_controllers) {
        Ok(s) => s,
        Err(err) => return Err(format!("failed to read enabled controllers: {}", err)),
    };
    let mut missing_controllers = Vec::new();
    for &controller in &["pids", "memory", "cpu"] {
        if !controllers.contains(controller) {
            missing_controllers.push(controller);
        }
    }
    if !missing_controllers.is_empty() {
        return Err(format!(
            "Required controllers are not enabled in cgroup.subtree_control: {:?}",
            missing_controllers
        ));
    }
    if let Err(err) = std::fs::write(cgroup_path.join("cgroup.procs"), "") {
        return Err(format!("cgroup can not be joined or left: {}", err));
    }
    Ok(())
}

fn find_lca<'a>(a: &'a [String], b: &'a [String]) -> &'a [String] {
    let n1 = a.len();
    let n2 = b.len();
    let n = std::cmp::min(n1, n2);

    for i in 0..n {
        if a[i] != b[i] {
            return &a[..i];
        }
    }
    if n1 < n2 { a } else { b }
}

/// `crate::check()` on linux
pub fn check() -> Result<(), String> {
    if detect_cgroup_version() == CgroupVersion::V1 {
        if unsafe { libc::geteuid() } != 0 {
            return Err("Root is required to use legacy cgroups".to_string());
        }
        return Ok(());
    }

    let sandbox_group = get_sandbox_chgroup_path();
    let current_group = get_current_cgroup_path();
    let lca = find_lca(&sandbox_group, &current_group);
    for group in &[&sandbox_group, lca] {
        if let Err(msg) = check_has_cgroup_access(&group) {
            return Err(format!("Access denied to cgroup {:?}: {}", group, msg));
        }
    }

    Ok(())
}
