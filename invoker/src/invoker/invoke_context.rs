use crate::{
    err::{self, Error},
    invoker::{CommandInterp, Paths},
    InvokeRequest,
};
use cfg::Config;
use minion::HandleWrapper;
use slog::{debug, Logger};
use snafu::ResultExt;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

/// Contains data for invokation
pub struct InvokeContext<'a> {
    pub minion_backend: &'a dyn minion::Backend,
    pub cfg: &'a Config,
    pub logger: &'a Logger,
    pub req: &'a InvokeRequest,
}

impl<'a> Clone for InvokeContext<'a> {
    fn clone<'b>(&'b self) -> InvokeContext<'a> {
        InvokeContext {
            minion_backend: self.minion_backend,
            cfg: self.cfg,
            logger: self.logger,
            req: self.req,
        }
    }
}

impl<'a> InvokeContext<'a> {
    pub(crate) fn get_asset_path(&self, short_path: &str) -> PathBuf {
        self.cfg
            .sysroot
            .join("var/problems")
            .join(&self.req.problem.name)
            .join("assets")
            .join(&short_path)
    }

    pub(crate) fn create_sandbox(
        &self,
        limits: &cfg::Limits,
        paths: &Paths,
    ) -> Result<minion::DominionRef, Error> {
        let mut exposed_paths = vec![];
        let toolchains_dir = self.cfg.sysroot.join("opt");
        let opt_items = fs::read_dir(&toolchains_dir).context(err::Io {})?;
        for item in opt_items {
            let item = item.context(err::Io {})?;
            let item_type = item.file_type().context(err::Io {})?;
            if !item_type.is_dir() {
                panic!("couldn't link child chroot, because it contains toplevel-item `{:?}`, which is not directory", item.file_name());
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
            memory_limit: (limits.memory * crate::invoker::MEGABYTE) as _,
            exposed_paths,
            isolation_root: paths.chroot_dir(),
            time_limit,
        };

        self.minion_backend
            .new_dominion(dominion_options)
            .context(err::Minion {})
    }

    pub(crate) fn get_common_interpolation_dict(&self) -> HashMap<String, OsString> {
        let submission = &self.req.submission;
        let mut dict = HashMap::new();
        dict.insert("System.Name".to_string(), OsString::from("JJS"));
        dict.insert("Invoker.Id".to_string(), OsString::from("inv"));
        dict.insert(
            "Submission.SourceFilePath".to_string(),
            PathBuf::from("/jjs")
                .join(&submission.toolchain.filename)
                .into_os_string(),
        );
        dict.insert("Submission.BinaryFilePath".to_string(), "/jjs/build".into());
        dict.insert(
            "Submission.ToolchainName".to_string(),
            submission.toolchain.name.clone().into(),
        );
        dict.insert(
            "Submission.Id".to_string(),
            submission.id.to_string().into(),
        );
        for (k, v) in submission.metadata.iter() {
            dict.insert(format!("Submission.Metadata.{}", k), v.clone().into());
        }
        dict
    }

    //
    // Command builders
    //

    pub(crate) fn command_builder_set_from_command(
        &self,
        cmd: &mut minion::Command,
        command: CommandInterp,
    ) {
        cmd.path(&command.argv[0]);
        cmd.args(&command.argv[1..]);
        cmd.envs(&command.env);
    }

    pub(crate) fn command_builder_set_stdio(
        &self,
        cmd: &mut minion::Command,
        stdout_path: &Path,
        stderr_path: &Path,
    ) {
        let stdout_file = fs::File::create(stdout_path).expect("io error");

        let stderr_file = fs::File::create(stderr_path).expect("io error");
        cmd.stdout(minion::OutputSpecification::RawHandle(unsafe {
            HandleWrapper::from(stdout_file)
        }));

        cmd.stderr(minion::OutputSpecification::RawHandle(unsafe {
            HandleWrapper::from(stderr_file)
        }));
    }

    pub(crate) fn log_execute_command(&self, command_interp: &CommandInterp) {
        debug!(self.logger, "executing command"; "command" => ?command_interp);
    }
}
