//! This library is responsible for fetching problem packages

pub use registry::Registry;

mod registry;

use anyhow::Context;
use std::{collections::HashMap, path::PathBuf};

// TODO: cache expiration, checksum, etc
/// Stores cached problem information
struct ProblemCache {
    /// Maps problem name to problem cache.
    items: HashMap<String, ProblemCacheItem>,
}

impl ProblemCache {
    fn new() -> ProblemCache {
        ProblemCache {
            items: HashMap::new(),
        }
    }
}
struct ProblemCacheItem {
    assets: PathBuf,
    manifest: pom::Problem,
}

pub struct Loader {
    registries: Vec<Box<dyn Registry>>,
    cache: tokio::sync::Mutex<ProblemCache>,
    /// Each problem will be represented by ${cache_dir}/${problem_name}
    cache_dir: PathBuf,
}

impl Loader {
    pub async fn from_config(conf: &LoaderConfig, cache_dir: PathBuf) -> anyhow::Result<Loader> {
        let mut loader = Loader {
            registries: vec![],
            cache_dir: cache_dir.to_path_buf(),
            cache: tokio::sync::Mutex::new(ProblemCache::new()),
        };
        if let Some(fs) = &conf.fs {
            let fs_reg = registry::FsRegistry::new(fs.clone());
            loader.registries.push(Box::new(fs_reg));
        }
        if let Some(mongodb) = &conf.mongodb {
            let mongo_reg = registry::MongoRegistry::new(mongodb)
                .await
                .context("unable to initialize MongodbRegistry")?;
            loader.registries.push(Box::new(mongo_reg));
        }
        Ok(loader)
    }

    /// Tries to resolve problem named `problem_name` in all configured
    /// registries. On success, returns problem manifest to path to assets dir.
    pub async fn find(
        &self,
        problem_name: &str,
    ) -> anyhow::Result<Option<(pom::Problem, PathBuf)>> {
        let mut cache = self.cache.lock().await;
        if let Some(cached_info) = cache.items.get(problem_name) {
            return Ok(Some((
                cached_info.manifest.clone(),
                cached_info.assets.clone(),
            )));
        }
        // cache for this problem not found, let's load it.
        let assets_path = self.cache_dir.join(problem_name);
        for registry in &self.registries {
            if let Some(manifest) = registry.get_problem(problem_name, &assets_path).await? {
                cache.items.insert(
                    problem_name.to_string(),
                    ProblemCacheItem {
                        manifest: manifest.clone(),
                        assets: assets_path.clone(),
                    },
                );
                return Ok(Some((manifest, assets_path)));
            }
        }
        // no registry knows about this problem
        Ok(None)
    }
}

/// Used in [`from_config`](Loader::from_config) constructor
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LoaderConfig {
    fs: Option<std::path::PathBuf>,
    mongodb: Option<String>,
}
