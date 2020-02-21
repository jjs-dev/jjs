use crate::worker::{Command, InvokeRequest};
use anyhow::{bail, Context};
use slog_scope::debug;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) fn create_sandbox(
    req: &InvokeRequest,
    test_id: Option<u32>,
    backend: &dyn minion::Backend,
) -> anyhow::Result<minion::DominionRef> {
    let mut exposed_paths = vec![];
    let toolchains_dir = &req.toolchains_dir;
    let opt_items = fs::read_dir(&toolchains_dir).context("failed to list toolchains sysroot")?;
    for item in opt_items {
        let item = item.context("failed to stat toolchains sysroot item")?;
        let item_type = item
            .file_type()
            .context("failed to get toolchain sysroot item file type")?;
        if !item_type.is_dir() {
            bail!(
                "couldn't link child chroot, because it contains toplevel item `{}`, which is not directory",
                item.file_name().to_string_lossy()
            );
        }
        let name = item.file_name();
        let peo = minion::PathExpositionOptions {
            src: toolchains_dir.join(&name),
            dest: PathBuf::from(&name),
            access: minion::DesiredAccess::Readonly,
        };
        exposed_paths.push(peo)
    }
    let out_dir = req.step_dir(test_id);
    std::fs::create_dir_all(&out_dir).context("failed to create step directory")?;
    std::fs::create_dir_all(out_dir.join("data")).context("failed to create shared directory")?;
    exposed_paths.push(minion::PathExpositionOptions {
        src: out_dir.join("data"),
        dest: PathBuf::from("/jjs"),
        access: minion::DesiredAccess::Full,
    });
    let limits = if let Some(test_id) = test_id {
        req.problem.tests[(test_id - 1) as usize].limits
    } else {
        req.compile_limits
    };
    let cpu_time_limit = Duration::from_millis(limits.time() as u64);
    let real_time_limit = Duration::from_millis(limits.time() * 3 as u64);
    std::fs::create_dir(out_dir.join("root")).context("failed to create chroot dir")?;
    // TODO adjust integer types
    let dominion_options = minion::DominionOptions {
        max_alive_process_count: limits.process_count() as _,
        memory_limit: limits.memory() as _,
        exposed_paths,
        isolation_root: out_dir.join("root"),
        cpu_time_limit,
        real_time_limit,
    };

    backend
        .new_dominion(dominion_options)
        .context("failed to create minion dominion")
}

pub(crate) fn log_execute_command(command_interp: &Command) {
    debug!("executing command"; "command" => ?command_interp);
}

pub(crate) fn command_set_from_inv_req(cmd: &mut minion::Command, command: &Command) {
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
