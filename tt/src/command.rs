use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    argv: Vec<String>,
    exe: String,
    cwd: Option<String>,
    env: HashMap<String, String>,
}

impl Command {
    pub fn to_std_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.exe);
        cmd.args(self.argv.iter());
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }
        cmd.envs(self.env.iter());
        cmd
    }

    pub fn to_string_pretty(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        if let Some(cwd) = &self.cwd {
            write!(out, "cd {} && ", cwd).unwrap();
        }
        for (k, v) in &self.env {
            write!(out, "{}={} ", k, v).unwrap();
        }
        write!(out, "{}", &self.exe).unwrap();
        for arg in &self.argv {
            write!(out, " {}", arg).unwrap();
        }
        out
    }

    pub fn run_quiet(&mut self) {
        use std::{os::unix::process::ExitStatusExt, process::exit};
        let mut s = self.to_std_command();
        let out = s.output().expect("couldn't spawn");
        let status = out.status;
        if status.success() {
            return;
        }
        eprintln!("error: child returned error");

        let exit_code = if status.code().is_some() {
            format!("normal: {}", status.code().unwrap())
        } else {
            format!("signaled: {}", status.signal().unwrap())
        };
        eprintln!(
            "testgen did not finished successfully (exit code {})",
            exit_code
        );

        eprintln!("command: `{}`", self);
        eprintln!("child stdout:\n{}", String::from_utf8_lossy(&out.stdout));
        eprintln!("child stderr:\n{}", String::from_utf8_lossy(&out.stderr));
        exit(1);
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.to_string_pretty())
    }
}

impl Command {
    pub fn new(s: &str) -> Command {
        let s = s.to_string();
        Command {
            exe: s,
            argv: vec![],
            cwd: None,
            env: HashMap::new(),
        }
    }

    pub fn arg(&mut self, a: &str) -> &mut Self {
        self.argv.push(a.to_string());
        self
    }

    pub fn env(&mut self, key: &str, value: &str) -> &mut Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    pub fn current_dir(&mut self, cwd: &str) -> &mut Self {
        self.cwd.replace(cwd.to_string());
        self
    }
}
