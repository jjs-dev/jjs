use crate::{entities::Entity, loader::EntitiesData, Loader};
use anyhow::Context;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    path::Path,
    sync::Arc,
};

pub struct LoaderBuilder(crate::loader::EntitiesData);

impl LoaderBuilder {
    pub fn new() -> LoaderBuilder {
        LoaderBuilder(EntitiesData {
            entities: (HashMap::new()),
        })
    }

    /// Saves new entity or updates existing. Returns true if entity was
    /// created.
    ///
    /// Useful for tests
    pub fn put<T: Entity>(&mut self, entity: T) -> bool {
        self.0
            .entities
            .entry(TypeId::of::<T>())
            .or_default()
            .insert(
                entity.name().to_string(),
                Box::new(entity) as Box<dyn Any + Send + Sync>,
            )
            .is_none()
    }

    pub fn into_inner(self) -> Loader {
        Loader::from_inner(Arc::new(self.0))
    }

    pub fn load_entity_from_file<T: Entity>(&mut self, path: &Path) -> anyhow::Result<()> {
        let extension = path.extension().and_then(|s| s.to_str());
        let extension = match extension {
            Some(ext) => ext,
            None => anyhow::bail!("missing extension"),
        };
        if !["yml", "yaml"].contains(&extension) {
            anyhow::bail!("unknown extension: {}", extension);
        }
        let content = std::fs::read(path).context("failed to read entity manifest")?;

        let mut entity: T = serde_yaml::from_slice(&content).context("parse error")?;
        entity.postprocess().context("postprocessing failure")?;
        self.put(entity);
        Ok(())
    }

    fn load_entities_from_dir<T: Entity>(&mut self, dir: &Path) -> anyhow::Result<()> {
        for item in dir.read_dir().context("failed to read dir")? {
            let item = item?;
            self.load_entity_from_file::<T>(&item.path())
                .with_context(|| format!("failed to load entity from {}", item.path().display()))?;
        }

        Ok(())
    }

    pub fn load_from_dir(&mut self, dir: &Path) -> anyhow::Result<()> {
        self.load_entities_from_dir::<crate::Toolchain>(&dir.join("toolchains"))?;
        self.load_entities_from_dir::<crate::Contest>(&dir.join("contests"))?;
        Ok(())
    }

    pub fn load_from_data_dir(&mut self, jjs_data_dir: &Path) -> anyhow::Result<()> {
        let entities_dir = jjs_data_dir.join("etc/objects");
        self.load_from_dir(&entities_dir)
            .with_context(|| format!("failed to load entities from {}", entities_dir.display()))?;
        Ok(())
    }
}

impl Default for LoaderBuilder {
    fn default() -> Self {
        LoaderBuilder::new()
    }
}
