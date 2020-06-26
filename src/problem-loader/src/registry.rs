//! defines Registry trait and several registries

use anyhow::Context as _;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Single problem source.
/// `problem-loader` itself is just abstraction for group of
/// registries.
#[async_trait]
pub trait Registry: Send + Sync {
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    /// Tries to fetch problem manifest and download assets to given path.
    /// Returns None if problem was not found.
    async fn get_problem(
        &self,
        problem_name: &str,
        assets_path: &Path,
    ) -> anyhow::Result<Option<pom::Problem>>;
}

/// Resolves problems from filesystem
#[derive(Debug)]
pub struct FsRegistry {
    /// Directory containing all problems
    problems_dir: PathBuf,
}

impl FsRegistry {
    pub fn new(problems_dir: PathBuf) -> FsRegistry {
        FsRegistry { problems_dir }
    }
}

#[async_trait]
impl Registry for FsRegistry {
    #[instrument]
    async fn get_problem(
        &self,
        problem_name: &str,
        dest_path: &Path,
    ) -> anyhow::Result<Option<pom::Problem>> {
        let problem_dir = self.problems_dir.join(problem_name);
        let manifest_path = problem_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Ok(None);
        }
        let manifest = tokio::fs::read(manifest_path).await?;
        let manifest = serde_json::from_slice(&manifest).context("invalid problem manifest")?;
        let assets_dir = problem_dir.join("assets");
        let dest_path = dest_path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            fs_extra::dir::copy(&assets_dir, &dest_path, &fs_extra::dir::CopyOptions::new())?;
            Ok::<_, anyhow::Error>(())
        })
        .await
        .unwrap()?;
        Ok(Some(manifest))
    }
}

/// Resolves problems via MongoDB
pub struct MongoRegistry {
    collection: mongodb::Collection,
}

impl std::fmt::Debug for MongoRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MongoRegistry")
            .field("collection", &"..")
            .finish()
    }
}

impl MongoRegistry {
    #[instrument]
    pub async fn new(connection_string: &str) -> anyhow::Result<MongoRegistry> {
        let client = mongodb::Client::with_uri_str(connection_string)
            .await
            .context("database is not available")?;
        let database = client.database("jjs");
        let collection = database.collection("problems");
        Ok(MongoRegistry { collection })
    }
}

#[async_trait]
impl Registry for MongoRegistry {
    #[instrument]
    async fn get_problem(
        &self,
        problem_name: &str,
        target_path: &Path,
    ) -> anyhow::Result<Option<pom::Problem>> {
        // at first, let's find document about this problem
        let filter = {
            let mut filter = bson::Document::new();
            filter.insert("problem-name", problem_name);
            filter
        };
        let doc = self
            .collection
            .find_one(filter, None)
            .await
            .context("problem document lookup failure")?;
        let doc = match doc {
            Some(d) => d,
            // if we got None, problem not found
            None => return Ok(None),
        };
        tracing::info!("problem found");
        let manifest = doc
            .get_binary_generic("manifest")
            .context("storage schema violation for field `manifest`")?;
        let manifest = serde_json::from_slice(&manifest).context("invalid problem manifest")?;

        let compressed_assets = std::mem::take(
            std::convert::identity(doc)
                .get_binary_generic_mut("assets")
                .context("storage schema violation for field `assets`")?,
        );
        // now we must unpack `compressed_assets` to target_path

        let target_path = target_path.to_path_buf();
        let cur_span = tracing::Span::current();
        tokio::task::spawn_blocking(move || {
            let _enter = cur_span.enter();
            let decoder = flate2::bufread::GzDecoder::new(compressed_assets.as_slice());
            let mut archive = tar::Archive::new(decoder);
            tracing::info!(compressed_size=compressed_assets.len(), path=%target_path.display(), "Unpacking problem");
            archive.unpack(target_path.join("assets"))
        })
        .await
        .unwrap()
        .context("failed to unpack")?;

        Ok(Some(manifest))
    }
}
