use glob::glob;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::path::{Path, PathBuf};

#[derive(Debug, Snafu)]
pub enum Error {
    NoMatch,
    Io {
        error: std::io::Error,
        path: Option<PathBuf>,
    },
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io {
            error: e,
            path: None,
        }
    }
}

impl From<glob::GlobError> for Error {
    fn from(e: glob::GlobError) -> Self {
        let path = e.path().to_owned();
        let error = e.into_error();
        Error::Io {
            error,
            path: Some(path),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct MagicBuildParams<'a> {
    pub path: &'a Path,
    pub out: &'a Path,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    argv: Vec<String>,
    exe: String,
}

impl Command {
    pub fn to_std_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.exe);
        cmd.args(self.argv.iter());
        cmd
    }

    pub fn to_string_pretty(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        write!(out, "{}", &self.exe).unwrap();
        for arg in &self.argv {
            write!(out, " {}", arg).unwrap();
        }
        out
    }
}

impl Command {
    fn new(s: &str) -> Command {
        let s = s.to_string();
        Command {
            exe: s,
            argv: vec![],
        }
    }

    fn arg(&mut self, a: &str) -> &mut Self {
        self.argv.push(a.to_string());
        self
    }
}

#[derive(Debug)]
pub struct MagicBuildSpec {
    pub build: Vec<Command>,
    pub run: Command,
}

fn mbuild_single_cpp(params: MagicBuildParams) -> Result<Option<MagicBuildSpec>> {
    let cpp_files_glob = format!("{}/**/*.cpp", params.path.display());
    let cpp_files: Result<Vec<_>> = glob(&cpp_files_glob)
        .expect("internal error: invalid pattern")
        .map(|item| item.map_err(Error::from))
        .collect();
    let cpp_files = cpp_files?;
    if cpp_files.len() != 1 {
        return Ok(None);
    }

    let file = &cpp_files[0];

    let mut cmd = Command::new("g++");
    cmd.arg("-std=c++17").arg(file.to_str().unwrap());

    let out_file_name = format!("{}/bin", params.out.display());
    cmd.arg("-o").arg(&out_file_name);

    let run_cmd = Command::new(&out_file_name);

    let spec = MagicBuildSpec {
        build: vec![cmd],
        run: run_cmd,
    };

    Ok(Some(spec))
}

pub fn magic_build(params: MagicBuildParams) -> Result<MagicBuildSpec> {
    mbuild_single_cpp(params)
        .transpose()
        .unwrap_or(Err(Error::NoMatch))
}
