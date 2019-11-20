use crate::invoke_env::InvokeEnv;
use pom::FileRef;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// Provides data and utilities for invocation
pub(crate) trait InvokeContext {
    fn resolve_asset(&self, short_path: &pom::FileRef) -> PathBuf;
    fn env(&self) -> &InvokeEnv;
}

pub(crate) struct MainInvokeContext<'a> {
    pub(crate) env: InvokeEnv<'a>,
}

impl<'a> InvokeContext for MainInvokeContext<'a> {
    fn resolve_asset(&self, short_path: &FileRef) -> PathBuf {
        let root: Cow<Path> = match short_path.root {
            pom::FileRefRoot::Problem => self.env().problem_root().join("assets").into(),
            pom::FileRefRoot::System => (&self.env().cfg.install_dir).into(),
            pom::FileRefRoot::Root => Path::new("/").into(),
        };

        root.join(&short_path.path)
    }

    fn env(&self) -> &InvokeEnv {
        &self.env
    }
}
