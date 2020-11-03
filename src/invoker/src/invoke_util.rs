use anyhow::Context;
use judging_apis::invoke::{Command, EnvVarValue, InvokeRequest};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::fs;
use tracing::{debug, error};

pub(crate) struct Sandbox {
    pub(crate) sandbox: Box<dyn minion::erased::Sandbox>,
    umount: Option<PathBuf>,
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Some(p) = self.umount.take() {
            if let Err(err) = nix::mount::umount2(&p, nix::mount::MntFlags::MNT_DETACH) {
                error!("Leaking tmpfs at {}: umount2 failed: {}", p.display(), err)
            } else {
                debug!("Successfully destroyed tmpfs at {}", p.display())
            }
        } else {
            panic!("TODO, REMOVE: winda??")
        }
    }
}

static DEFAULT_HOST_MOUNTS: once_cell::sync::Lazy<Vec<String>> = once_cell::sync::Lazy::new(|| {
    vec![
        "usr".to_string(),
        "bin".to_string(),
        "lib".to_string(),
        "lib64".to_string(),
    ]
});

pub(crate) async fn create_sandbox(
    config: &crate::config::Config,
    req: &InvokeRequest,
    req_id: &str,
    backend: &dyn minion::erased::Backend,
    settings: judging_apis::invoke::Sandbox,
) -> anyhow::Result<Sandbox> {
    let mut shared_dirs = vec![];
    if config.host_toolchains {
        let dirs = config
            .expose_host_dirs
            .as_ref()
            .unwrap_or_else(|| &*DEFAULT_HOST_MOUNTS);
        for item in dirs {
            let item = format!("/{}", item);
            let shared_dir = minion::SharedDir {
                src: item.clone().into(),
                dest: item.into(),
                kind: minion::SharedDirKind::Readonly,
            };
            shared_dirs.push(shared_dir)
        }
    } else {
        let toolchain_dir = &req.toolchain_dir;
        let mut opt_items = fs::read_dir(&toolchain_dir)
            .await
            .context("failed to list toolchains sysroot")?;
        while let Some(item) = opt_items.next_entry().await? {
            let name = item.file_name();
            let shared_dir = minion::SharedDir {
                src: toolchain_dir.join(&name),
                dest: PathBuf::from(&name),
                kind: minion::SharedDirKind::Readonly,
            };
            shared_dirs.push(shared_dir)
        }
    }

    let work_dir = config.work_root.join(req_id);
    tokio::fs::create_dir_all(&work_dir)
        .await
        .context("failed to create working directory")?;
    let umount_path;
    #[cfg(target_os = "linux")]
    {
        let quota = settings.limits.work_dir_size();
        let quota = minion::linux::ext::Quota::bytes(quota);
        minion::linux::ext::make_tmpfs(&work_dir.join("data"), quota)
            .context("failed to set size limit on shared directory")?;
        umount_path = Some(work_dir.join("data"));
    }
    #[cfg(not(target_os = "linux"))]
    {
        umount_path = None;
    }
    shared_dirs.push(minion::SharedDir {
        src: work_dir.join("data"),
        dest: PathBuf::from("/jjs"),
        kind: minion::SharedDirKind::Full,
    });
    let cpu_time_limit = Duration::from_millis(settings.limits.time() as u64);
    let real_time_limit = Duration::from_millis(settings.limits.time() * 3 as u64);
    tokio::fs::create_dir(work_dir.join("root"))
        .await
        .context("failed to create chroot dir")?;
    // TODO adjust integer types
    let sandbox_options = minion::SandboxOptions {
        max_alive_process_count: settings.limits.process_count() as _,
        memory_limit: settings.limits.memory() as _,
        exposed_paths: shared_dirs,
        isolation_root: work_dir.join("root"),
        cpu_time_limit,
        real_time_limit,
    };
    let sandbox = backend
        .new_sandbox(sandbox_options)
        .context("failed to create minion dominion")?;
    Ok(Sandbox {
        sandbox,
        umount: umount_path,
    })
}

pub(crate) fn log_execute_command(command_interp: &Command) {
    debug!("executing command {:?}", command_interp);
}

pub(crate) fn command_set_from_judge_req(cmd: &mut minion::Command, command: &Command) {
    cmd.path(&command.argv[0]);
    cmd.args(&command.argv[1..]);
    cmd.envs(
        command
            .env
            .iter()
            .map(|(name, value)| -> std::ffi::OsString {
                match value {
                    EnvVarValue::Plain(p) => format!("{}={}", name, p).into(),
                    EnvVarValue::File(_) => unreachable!(),
                }
            }),
    );
}

pub(crate) async fn command_set_stdio(
    cmd: &mut minion::Command,
    stdout_path: &Path,
    stderr_path: &Path,
) {
    let stdout_file = fs::File::create(stdout_path).await.expect("io error");

    let stderr_file = fs::File::create(stderr_path).await.expect("io error");
    // Safety: std::fs::File owns it's handle
    unsafe {
        cmd.stdout(minion::OutputSpecification::handle_of(
            stdout_file.into_std().await,
        ));

        cmd.stderr(minion::OutputSpecification::handle_of(
            stderr_file.into_std().await,
        ));
    }
}
