use async_trait::async_trait;
use std::path::Path;
use tokio::stream::StreamExt;

#[derive(Copy, Clone)]
pub struct Context<'a> {
    pub data_dir: &'a Path,
}

#[derive(Copy, Clone)]
enum DataLayoutState {
    Exists,
    NotExists,
    Unknown,
}

pub struct DataLayout<'a> {
    cx: Context<'a>,
    state: DataLayoutState,
}

impl std::fmt::Display for DataLayout<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.state {
            DataLayoutState::Exists => write!(f, "exists and non-empty"),
            DataLayoutState::NotExists => write!(f, "does not exist"),
            DataLayoutState::Unknown => write!(f, "unexpected"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
}

#[async_trait]
impl<'a> crate::Component for DataLayout<'a> {
    type Error = Error;

    async fn state(&self) -> Result<crate::StateKind, Error> {
        Ok(match self.state {
            DataLayoutState::Exists => crate::StateKind::UpToDate,
            DataLayoutState::NotExists => crate::StateKind::Upgradable,
            DataLayoutState::Unknown => crate::StateKind::Errored,
        })
    }

    fn name(&self) -> &'static str {
        "data dir layout"
    }

    async fn upgrade(&self) -> Result<(), Error> {
        let base = self.cx.data_dir;
        tokio::fs::create_dir_all(base).await?;
        tokio::fs::create_dir(base.join("var")).await?;
        tokio::fs::create_dir(base.join("var/runs")).await?;
        tokio::fs::create_dir(base.join("var/problems")).await?;
        tokio::fs::create_dir(base.join("etc")).await?;
        tokio::fs::create_dir(base.join("etc/pki")).await?;
        tokio::fs::create_dir(base.join("etc/objects")).await?;
        tokio::fs::create_dir(base.join("etc/objects/toolchains")).await?;
        tokio::fs::create_dir(base.join("etc/objects/contests")).await?;
        tokio::fs::create_dir(base.join("opt")).await?;
        tokio::fs::create_dir(base.join("tmp")).await?;
        Ok(())
    }
}

async fn find_state(cx: Context<'_>) -> Result<DataLayoutState, Error> {
    match tokio::fs::metadata(&cx.data_dir).await {
        Ok(metadata) => {
            if metadata.is_file() {
                return Ok(DataLayoutState::Unknown);
            }
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                return Ok(DataLayoutState::NotExists);
            } else {
                return Err(err.into());
            }
        }
    }
    let mut items = tokio::fs::read_dir(&cx.data_dir).await?;
    let mut item_names = Vec::new();
    while let Some(item) = items.next().await {
        item_names.push(item?.file_name().to_string_lossy().to_string());
    }
    if item_names.contains(&"etc".to_string()) && item_names.contains(&"var".to_string()) {
        Ok(DataLayoutState::Exists)
    } else {
        Ok(DataLayoutState::NotExists)
    }
}

pub async fn analyze<'a>(cx: Context<'a>) -> Result<DataLayout<'a>, Error> {
    let state = find_state(cx).await?;
    Ok(DataLayout { cx, state })
}
