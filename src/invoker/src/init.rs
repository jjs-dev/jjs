//! platform-specific initialization
use anyhow::{bail, Context};
use nix::sched::CloneFlags;
fn check_system() -> anyhow::Result<()> {
    if let Some(err) = minion::check() {
        bail!("invoker is not able to test runs: {}", err);
    }
    Ok(())
}

fn unshare_mount_namespace() -> anyhow::Result<()> {
    nix::sched::unshare(CloneFlags::CLONE_NEWNS).context("unshare() fail")
}

fn unshare_user_namespace() -> anyhow::Result<()> {
    if nix::unistd::getuid().is_root() {
        return Ok(());
    }
    let uid = nix::unistd::getuid().as_raw();
    let gid = nix::unistd::getgid().as_raw();
    let uid_mapping = format!("0 {} 1", uid);
    let gid_mapping = format!("0 {} 1", gid);
    nix::sched::unshare(CloneFlags::CLONE_NEWUSER).context("unshare() fail")?;
    std::fs::write("/proc/self/setgroups", "deny").context("failed to deny setgroups()")?;
    std::fs::write("/proc/self/uid_map", uid_mapping).context("failed to setup uid mapping")?;
    std::fs::write("/proc/self/gid_map", gid_mapping).context("failed to setup gid mapping")?;
    nix::unistd::setuid(nix::unistd::Uid::from_raw(0)).expect("failed to become root user");
    nix::unistd::setgid(nix::unistd::Gid::from_raw(0)).expect("failed to join root group");
    Ok(())
}

fn unshare() -> anyhow::Result<()> {
    unshare_user_namespace().context("failed to unshare user ns")?;
    unshare_mount_namespace().context("failed to unshare mount ns")?;
    Ok(())
}

pub fn init() -> anyhow::Result<()> {
    check_system().context("system configuration problem detected")?;
    unshare().context("failed to create namespaces")?;
    Ok(())
}
