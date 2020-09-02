use crate::worker::{Command, LoweredJudgeRequest};
use anyhow::Context;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
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

pub(crate) fn create_sandbox(
    req: &LoweredJudgeRequest,
    test_id: Option<u32>,
    backend: &dyn minion::erased::Backend,
    config: &crate::config::JudgeConfig,
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
        let opt_items =
            fs::read_dir(&toolchain_dir).context("failed to list toolchains sysroot")?;
        for item in opt_items {
            let item = item.context("failed to stat toolchains sysroot item")?;
            let name = item.file_name();
            let shared_dir = minion::SharedDir {
                src: toolchain_dir.join(&name),
                dest: PathBuf::from(&name),
                kind: minion::SharedDirKind::Readonly,
            };
            shared_dirs.push(shared_dir)
        }
    }

    let limits = if let Some(test_id) = test_id {
        req.problem.tests[(test_id - 1) as usize].limits
    } else {
        req.compile_limits
    };
    let out_dir = req.step_dir(test_id);
    std::fs::create_dir_all(&out_dir).context("failed to create step directory")?;
    let umount_path;
    #[cfg(target_os = "linux")]
    {
        let quota = limits.work_dir_size();
        let quota = minion::linux::ext::Quota::bytes(quota);
        minion::linux::ext::make_tmpfs(&out_dir.join("data"), quota)
            .context("failed to set size limit on shared directory")?;
        umount_path = Some(out_dir.join("data"));
    }
    #[cfg(not(target_os = "linux"))]
    {
        umount_path = None;
    }
    shared_dirs.push(minion::SharedDir {
        src: out_dir.join("data"),
        dest: PathBuf::from("/jjs"),
        kind: minion::SharedDirKind::Full,
    });
    let cpu_time_limit = Duration::from_millis(limits.time() as u64);
    let real_time_limit = Duration::from_millis(limits.time() * 3 as u64);
    std::fs::create_dir(out_dir.join("root")).context("failed to create chroot dir")?;
    // TODO adjust integer types
    let sandbox_options = minion::SandboxOptions {
        max_alive_process_count: limits.process_count() as _,
        memory_limit: limits.memory() as _,
        exposed_paths: shared_dirs,
        isolation_root: out_dir.join("root"),
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
    cmd.envs(&command.env);
}

pub(crate) fn command_set_stdio(cmd: &mut minion::Command, stdout_path: &Path, stderr_path: &Path) {
    let stdout_file = fs::File::create(stdout_path).expect("io error");

    let stderr_file = fs::File::create(stderr_path).expect("io error");
    // Safety: std::fs::File owns it's handle
    unsafe {
        cmd.stdout(minion::OutputSpecification::handle_of(stdout_file));

        cmd.stderr(minion::OutputSpecification::handle_of(stderr_file));
    }
}
