use async_trait::async_trait;
use futures::future::FutureExt;
use std::path::Path;
use tokio::stream::StreamExt;

enum ConfigStateItem {
    Exists,
    CanCopy,
}

struct ConfigState {
    items: Vec<(String, ConfigStateItem)>,
}

#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub data_dir: &'a Path,
    pub install_dir: &'a Path,
}

pub struct Cfg<'a> {
    cx: Context<'a>,
    state: ConfigState,
}

impl std::fmt::Display for Cfg<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for item in &self.state.items {
            let state_description = match item.1 {
                ConfigStateItem::CanCopy => "copyable",
                ConfigStateItem::Exists => "already exists",
            };
            write!(f, "{}: {};", item.0, state_description)?;
        }
        Ok(())
    }
}

#[async_trait]
impl<'a> crate::Component for Cfg<'a> {
    type Error = Error;

    fn name(&self) -> &'static str {
        "configs"
    }

    async fn state(&self) -> Result<crate::StateKind, Error> {
        let mut state = crate::StateKind::UpToDate;
        for item in &self.state.items {
            if matches!(item.1, ConfigStateItem::CanCopy) {
                state = crate::StateKind::Upgradable;
            }
        }
        Ok(state)
    }

    async fn upgrade(&self) -> Result<(), Error> {
        for item in &self.state.items {
            if matches!(item.1, ConfigStateItem::CanCopy) {
                let src = self
                    .cx
                    .install_dir
                    .join(format!("example-config/{}", item.0));
                let dst = self.cx.data_dir.join(format!("etc/{}", item.0));
                tokio::fs::copy(src, dst).await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
}

fn do_list_available_configs<'a>(
    path: &'a Path,
    path_items: &'a mut Vec<String>,
    out: &'a mut Vec<String>,
) -> futures::future::BoxFuture<'a, std::io::Result<()>> {
    async move {
        if path.is_file() {
            out.push(path_items.join("/"));
            return Ok(());
        }
        let mut items = tokio::fs::read_dir(path).await?;
        while let Some(item) = items.next().await {
            let item = item?;
            let name = item.file_name();
            let name = &name;
            let name = name.to_str();
            let name = name.expect("file name in example-config/ is not utf-8");
            path_items.push(name.to_string());
            do_list_available_configs(&item.path(), path_items, out).await?;
            path_items.pop();
        }
        Ok(())
    }
    .boxed()
}

async fn list_available_configs(cx: Context<'_>) -> std::io::Result<Vec<String>> {
    let mut out = Vec::new();
    do_list_available_configs(
        &cx.install_dir.join("example-config"),
        &mut Vec::new(),
        &mut out,
    )
    .await?;
    Ok(out)
}

async fn detect_state(cx: Context<'_>) -> Result<ConfigState, Error> {
    let available_configs = list_available_configs(cx).await?;
    let base = cx.data_dir.join("etc");
    if !base.exists() {
        let mut items = Vec::new();
        for conf in available_configs {
            items.push((conf, ConfigStateItem::CanCopy));
        }
        return Ok(ConfigState { items });
    }
    let mut items = Vec::new();
    for conf in available_configs {
        let current_path = base.join(&conf);
        if !current_path.exists() {
            items.push((conf, ConfigStateItem::CanCopy));
        } else {
            items.push((conf, ConfigStateItem::Exists));
        }
    }
    Ok(ConfigState { items })
}

pub async fn analyze<'a>(cx: Context<'a>) -> Result<Cfg<'a>, Error> {
    let state = detect_state(cx).await?;
    Ok(Cfg { cx, state })
}
