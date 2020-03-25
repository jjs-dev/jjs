use std::path::{Path, PathBuf};
use tokio::stream::StreamExt;
struct ProblemsState {
    config_problems: Vec<String>,
    installable_problems: Vec<(String, PathBuf)>,
}

impl ProblemsState {
    fn extra<'a>(&'a self) -> impl Iterator<Item = (&'a str, &'a Path)> + 'a {
        self.installable_problems
            .iter()
            .filter(move |s| !self.config_problems.contains(&s.0))
            .map(|(s, p)| (s.as_str(), p.as_path()))
    }
}

#[derive(Copy, Clone)]
pub struct Context<'a> {
    pub data_dir: &'a Path,
    pub install_dir: &'a Path,
    pub paths: &'a [&'a Path],
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("problem name is not utf8")]
    ProblemNameNotUtf8,
    #[error("invalid problem.toml")]
    ParseToml {
        #[from]
        source: toml::de::Error,
    },
    #[error("cannot find problem name in problem.toml: {0}")]
    DetectProblemNameFromManifest(&'static str),
    #[error("ppc invokation failed")]
    Ppc,
}

fn detect_problem_name(manifest: &toml::Value) -> Result<&str, &'static str> {
    let root = manifest.as_table().ok_or("manifest is not table")?;
    let name = root.get("name").ok_or("field name missing")?;
    let name = name.as_str().ok_or("name is not string")?;
    Ok(name)
}

async fn detect_state(cx: Context<'_>) -> Result<ProblemsState, Error> {
    let mut config_problems = Vec::new();
    let problems_dir = cx.data_dir.join("var/problems");
    if problems_dir.exists() {
        let mut items = tokio::fs::read_dir(problems_dir).await?;
        while let Some(item) = items.next().await {
            let item = item?;
            let name = item.file_name();
            config_problems.push(name.to_str().ok_or(Error::ProblemNameNotUtf8)?.to_string());
        }
    }
    let mut installable_problems = Vec::new();
    for &path in cx.paths {
        let problem_manifest = path.join("problem.toml");
        let problem_manifest = tokio::fs::read(problem_manifest).await?;
        let problem_manifest = toml::from_slice(&problem_manifest)?;
        let problem_name = detect_problem_name(&problem_manifest)
            .map_err(|message| Error::DetectProblemNameFromManifest(message))?
            .to_string();
        installable_problems.push((problem_name, path.to_path_buf()));
    }

    Ok(ProblemsState {
        installable_problems,
        config_problems,
    })
}

pub struct Problems<'a> {
    cx: Context<'a>,
    state: ProblemsState,
}

impl std::fmt::Display for Problems<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut extra = self.state.extra().peekable();
        write!(f, "current: ")?;
        if self.state.config_problems.is_empty() {
            write!(f, "<none>")?;
        } else {
            let mut flag = false;
            for p in &self.state.config_problems {
                if flag {
                    write!(f, ", ")?;
                }
                flag = true;
                write!(f, "{}", p)?;
            }
        }
        write!(f, "; available: ")?;
        if extra.peek().is_some() {
            let mut flag = false;
            for item in extra {
                if flag {
                    write!(f, ", ")?;
                }
                flag = true;
                write!(f, "{}", item.0)?;
            }
        } else {
            write!(f, "<none>")?;
        }
        Ok(())
    }
}

pub async fn analyze<'a>(cx: Context<'a>) -> Result<Problems<'a>, Error> {
    let state = detect_state(cx).await?;
    Ok(Problems { cx, state })
}

#[async_trait::async_trait]
impl<'a> crate::Component for Problems<'a> {
    type Error = Error;

    async fn state(&self) -> Result<crate::StateKind, Self::Error> {
        if self.state.extra().next().is_some() {
            Ok(crate::StateKind::Upgradable)
        } else {
            Ok(crate::StateKind::UpToDate)
        }
    }

    fn name(&self) -> &'static str {
        "problems"
    }

    async fn upgrade(&self) -> Result<(), Self::Error> {
        for problem in self.state.extra() {
            // TODO: ppc should support parallel build
            let mut cmd = tokio::process::Command::new(self.cx.install_dir.join("bin/jjs-ppc"));
            cmd.arg("compile");
            cmd.arg("--pkg").arg(problem.1);
            let out_dir = self.cx.data_dir.join("var/problems").join(problem.0);
            tokio::fs::create_dir(&out_dir).await?;
            cmd.arg("--out").arg(out_dir);
            let status = cmd.status().await?;
            if !status.success() {
                return Err(Error::Ppc);
            }
        }
        Ok(())
    }
}
