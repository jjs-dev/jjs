use crate::{inter_api::Paths, invoker::CommandInterp, RunProps};
use anyhow::{bail, Context};
use slog_scope::debug;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) fn create_sandbox(
    cfg: &cfg::Config,
    limits: &cfg::Limits,
    paths: &Paths,
    backend: &dyn minion::Backend,
) -> anyhow::Result<minion::DominionRef> {
    let mut exposed_paths = vec![];
    let toolchains_dir = cfg.sysroot.join("opt");
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
    exposed_paths.push(minion::PathExpositionOptions {
        src: paths.share_dir(),
        dest: PathBuf::from("/jjs"),
        access: minion::DesiredAccess::Full,
    });
    let time_limit = Duration::from_millis(limits.time as u64);

    // TODO adjust integer types
    let dominion_options = minion::DominionOptions {
        max_alive_process_count: limits.process_count as _,
        memory_limit: limits.memory as _,
        exposed_paths,
        isolation_root: paths.chroot_dir(),
        time_limit,
    };

    backend
        .new_dominion(dominion_options)
        .context("failed to create minion dominion")
}

pub(crate) fn get_common_interpolation_dict(
    run_props: &RunProps,
    toolchain_cfg: &cfg::Toolchain,
) -> HashMap<String, OsString> {
    let mut dict = HashMap::new();
    dict.insert("Invoker.Id".to_string(), OsString::from("inv"));
    dict.insert(
        "Submission.SourceFilePath".to_string(),
        PathBuf::from("/jjs")
            .join(&toolchain_cfg.filename)
            .into_os_string(),
    );
    dict.insert("Submission.BinaryFilePath".to_string(), "/jjs/build".into());
    dict.insert(
        "Submission.ToolchainName".to_string(),
        toolchain_cfg.name.clone().into(),
    );
    dict.insert("Submission.Id".to_string(), run_props.id.to_string().into());
    for (k, v) in run_props.metadata.iter() {
        dict.insert(format!("Submission.Metadata.{}", k), v.clone().into());
    }
    dict
}

pub(crate) fn log_execute_command(command_interp: &CommandInterp) {
    debug!("executing command"; "command" => ?command_interp);
}

pub(crate) fn command_set_from_interp(cmd: &mut minion::Command, command: &CommandInterp) {
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
