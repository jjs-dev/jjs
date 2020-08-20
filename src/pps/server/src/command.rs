// TODO: split all this stuff to separate library
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    argv: Vec<OsString>,
    exe: OsString,
    cwd: Option<OsString>,
    env: Vec<(OsString, OsString)>,
}

impl Command {
    pub fn to_tokio_command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.exe);
        cmd.args(self.argv.iter());
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }
        cmd.envs(self.env.iter().cloned());
        cmd
    }

    pub fn to_string_pretty(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        if let Some(cwd) = &self.cwd {
            write!(out, "cd {} && ", cwd.to_string_lossy()).unwrap();
        }
        for (k, v) in &self.env {
            write!(out, "{}={} ", k.to_string_lossy(), v.to_string_lossy()).unwrap();
        }
        write!(out, "{}", self.exe.to_string_lossy()).unwrap();
        for arg in &self.argv {
            write!(out, " {}", arg.to_string_lossy()).unwrap();
        }
        out
    }

    pub async fn run_quiet(&mut self) -> anyhow::Result<std::process::Output> {
        use std::os::unix::process::ExitStatusExt;
        let mut s = self.to_tokio_command();
        let out = s.output().await.context("couldn't spawn")?;
        let status = out.status;
        if status.success() {
            return Ok(out);
        }

        let exit_code = if status.code().is_some() {
            format!("normal: {}", status.code().unwrap())
        } else {
            format!("signaled: {}", status.signal().unwrap())
        };
        anyhow::bail!(
            "child process did not finished successfully (exit code {})\ncommand: `{}`\nchild stdout:\n{}\nchild stderr:\n{}",
            exit_code,
            self,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.to_string_pretty())
    }
}

impl Command {
    pub fn new(s: impl AsRef<OsStr>) -> Command {
        Command {
            exe: s.as_ref().to_os_string(),
            argv: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    pub fn arg(&mut self, a: impl AsRef<OsStr>) -> &mut Self {
        self.argv.push(a.as_ref().to_os_string());
        self
    }

    pub fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        let key = key.as_ref().to_os_string();
        let value = value.as_ref().to_os_string();
        self.env.push((key, value));
        self
    }

    pub fn current_dir(&mut self, cwd: impl AsRef<OsStr>) -> &mut Self {
        self.cwd.replace(cwd.as_ref().to_os_string());
        self
    }
}
