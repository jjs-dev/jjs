use crate::{
    Backend, ChildProcess, ChildProcessOptions, DominionRef, InputSpecification,
    OutputSpecification, StdioSpecification,
};
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

/// Child process builder
#[derive(Default)]
pub struct Command {
    dominion: Option<DominionRef>,
    exe: Option<PathBuf>,
    argv: Vec<OsString>,
    env: HashMap<OsString, OsString>,
    stdin: Option<InputSpecification>,
    stdout: Option<OutputSpecification>,
    stderr: Option<OutputSpecification>,
    current_dir: Option<PathBuf>,
}

impl Command {
    pub fn build(&self) -> Option<ChildProcessOptions> {
        let create_default_in_channel = || InputSpecification::Empty;
        let create_default_out_channel = || OutputSpecification::Ignore;
        let opts = ChildProcessOptions {
            path: self.exe.clone()?,
            arguments: self.argv.clone(),
            environment: self.env.clone(),
            dominion: self.dominion.clone()?,
            stdio: StdioSpecification {
                stdin: self.stdin.clone().unwrap_or_else(create_default_in_channel),
                stdout: self
                    .stdout
                    .clone()
                    .unwrap_or_else(create_default_out_channel),
                stderr: self
                    .stderr
                    .clone()
                    .unwrap_or_else(create_default_out_channel),
            },
            pwd: self.current_dir.clone().unwrap_or_else(|| "/".into()),
        };
        Some(opts)
    }

    pub fn new() -> Command {
        Default::default()
    }

    pub fn spawn(&self, backend: &dyn Backend) -> crate::Result<Box<dyn ChildProcess>> {
        let options = self
            .build()
            .expect("spawn() was requested, but required fields were not set");
        backend.spawn(options)
    }

    pub fn dominion(&mut self, dominion: DominionRef) -> &mut Self {
        self.dominion.replace(dominion);
        self
    }

    pub fn path<S: AsRef<Path>>(&mut self, path: S) -> &mut Self {
        self.exe.replace(path.as_ref().to_path_buf());
        self
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, a: S) -> &mut Self {
        self.argv.push(a.as_ref().to_os_string());
        self
    }

    pub fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self {
        self.argv
            .extend(args.into_iter().map(|s| s.as_ref().to_os_string()));
        self
    }

    pub fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.env
            .insert(key.as_ref().to_os_string(), value.as_ref().to_os_string());
        self
    }

    pub fn envs(
        &mut self,
        items: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> &mut Self {
        self.env.extend(
            items
                .into_iter()
                .map(|(k, v)| (k.as_ref().to_os_string(), v.as_ref().to_os_string())),
        );
        self
    }

    pub fn current_dir<S: AsRef<Path>>(&mut self, a: S) -> &mut Self {
        self.current_dir.replace(a.as_ref().to_path_buf());
        self
    }

    pub fn stdin(&mut self, stdin: InputSpecification) -> &mut Self {
        self.stdin.replace(stdin);
        self
    }

    pub fn stdout(&mut self, stdout: OutputSpecification) -> &mut Self {
        self.stdout.replace(stdout);
        self
    }

    pub fn stderr(&mut self, stderr: OutputSpecification) -> &mut Self {
        self.stderr.replace(stderr);
        self
    }
}
