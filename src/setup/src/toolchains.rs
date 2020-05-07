use async_trait::async_trait;
use std::{ffi::OsStr, path::Path};
use tokio::stream::StreamExt;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("string is not utf8: {}", _0.to_string_lossy())]
    Utf8(std::ffi::OsString),
    #[error("listing toolchains")]
    ListTcs(#[from] ListTcsError),
    #[error("jjs-configure-toolchains failed: code={code:?}")]
    ConfigureTcs { code: Option<i32> },
    #[error("illegal file name")]
    BadFileName,
}

#[derive(Copy, Clone)]
pub struct Context<'a> {
    pub install_dir: &'a Path,
    pub data_dir: &'a Path,
    pub filter: &'a (dyn Fn(&str) -> bool + Send + Sync),
    pub custom_argv: &'a [&'a OsStr],
    pub strategies: &'a [&'a str],
}

pub struct Toolchains<'a> {
    cx: Context<'a>,
    state: TcsState,
}

impl std::fmt::Display for Toolchains<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "installed: {:?}, installable: {:?}",
            &self.state.installed, &self.state.extra
        )
    }
}

#[derive(Debug)]
struct TcsState {
    installed: Vec<String>,
    extra: Vec<String>,
}

#[async_trait]
impl<'a> crate::Component for Toolchains<'a> {
    type Error = Error;

    fn name(&self) -> &'static str {
        "toolchains"
    }

    async fn state(&self) -> Result<crate::StateKind, Error> {
        if self.state.extra.is_empty() {
            Ok(crate::StateKind::UpToDate)
        } else {
            Ok(crate::StateKind::Upgradable)
        }
    }

    async fn upgrade(&self) -> Result<(), Error> {
        let mut cmd =
            tokio::process::Command::new(self.cx.install_dir.join("bin/jjs-configure-toolchains"));

        let templates_dir = self.cx.install_dir.join("toolchains");
        let target_dir = self.cx.data_dir;
        cmd.arg(templates_dir).arg(target_dir);
        for tc in &self.state.extra {
            cmd.arg("--toolchains").arg(tc);
        }
        for &strategy in self.cx.strategies {
            cmd.arg("--strategies").arg(strategy);
        }
        for &extra_arg in self.cx.custom_argv {
            cmd.arg(extra_arg);
        }
        let status = cmd.status().await?;
        if !status.success() {
            return Err(Error::ConfigureTcs {
                code: status.code(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ListTcsError {
    #[error("execute jjs-configure-toolchains")]
    ExecConfigureToolchains(#[source] std::io::Error),
    #[error(
        "jjs-configure-toolchains returned error: code={code:?} stdout={stdout} stderr={stderr}"
    )]
    ConfigureToolchainsError {
        code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    #[error("parsing jjs-configure-toolchains output")]
    Parse(#[from] serde_json::Error),
}

async fn list_all_toolchains(cx: Context<'_>) -> Result<Vec<String>, ListTcsError> {
    let mut cmd = tokio::process::Command::new(cx.install_dir.join("bin/jjs-configure-toolchains"));
    cmd.env("__JJS", "print-usable-toolchains");
    cmd.arg(cx.install_dir.join("toolchains"));
    cmd.arg(cx.data_dir);
    let out = cmd
        .output()
        .await
        .map_err(ListTcsError::ExecConfigureToolchains)?;
    if !out.status.success() {
        return Err(ListTcsError::ConfigureToolchainsError {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        });
    }
    let data = serde_json::from_slice(&out.stdout)?;
    Ok(data)
}

async fn detect_state(cx: Context<'_>) -> Result<TcsState, Error> {
    let toolchains_config_dir = cx.data_dir.join("etc/objects/toolchains");
    let mut installed_toolchains = Vec::new();
    if toolchains_config_dir.exists() {
        let mut dir_contents = tokio::fs::read_dir(&toolchains_config_dir).await?;
        while let Some(item) = dir_contents.next().await {
            let name = item?.path();
            let name = name.file_stem().ok_or(Error::BadFileName)?;
            let name = match name.to_str() {
                Some(name) => name,
                None => return Err(Error::Utf8(name.to_os_string())),
            };
            installed_toolchains.push(name.to_string());
        }
    }
    let available_toolchains = list_all_toolchains(cx).await?;

    let mut extra = Vec::new();
    for tc in &available_toolchains {
        if !(cx.filter)(tc.as_str()) {
            continue;
        }
        if !installed_toolchains.contains(tc) {
            extra.push(tc.clone());
        }
    }

    Ok(TcsState {
        installed: installed_toolchains,
        extra,
    })
}

pub async fn analyze<'a>(cx: Context<'a>) -> Result<Toolchains<'a>, Error> {
    let state = detect_state(cx).await?;
    Ok(Toolchains { state, cx })
}
