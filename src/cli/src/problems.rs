//! Commands for problem management
use anyhow::Context as _;
use client::prelude::*;
use humansize::FileSize;
use std::{io::Write, path::PathBuf};

#[derive(clap::Clap)]
pub(crate) struct Opt {
    /// Problem package path
    #[clap(long)]
    pkg: PathBuf,
}

pub(crate) async fn exec(opt: &Opt, api: &client::ApiClient) -> anyhow::Result<()> {
    println!("loading assets");

    let assets_archive = {
        let assets_path = opt.pkg.join("assets");
        tokio::task::spawn_blocking(move || {
            let mut builder = tar::Builder::new(Vec::new());
            builder
                .append_dir_all("", assets_path)
                .context("failed to build tar")?;

            let data = builder.into_inner().context("failed to finalize tar")?;

            Ok::<_, anyhow::Error>(data)
        })
        .await
        .unwrap()
        .context("generating tarball failed")?
    };
    println!(
        "assets loaded: {}",
        assets_archive
            .len()
            .file_size(humansize::file_size_opts::BINARY)
            .unwrap()
    );

    println!("Compressing assets");
    let compressed_assets = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let mut compressor = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        compressor.write_all(&assets_archive)?;
        compressor
            .finish()
            .context("compression finalization failed")
    })
    .await
    .unwrap()
    .context("compression failed")?;
    println!(
        "assets compressed: {}",
        compressed_assets
            .len()
            .file_size(humansize::file_size_opts::BINARY)
            .unwrap()
    );
    let problem_manifest_path = opt.pkg.join("manifest.json");
    let problem_manifest_data = tokio::fs::read_to_string(problem_manifest_path)
        .await
        .context("failed to open problem manifest")?;
    let problem_manifest: pom::Problem =
        serde_json::from_str(&problem_manifest_data).context("invalid manifest")?;
    client::models::Misc::put_problem()
        .problem_id(problem_manifest.name.clone())
        .problem_manifest(problem_manifest_data)
        .problem_assets(base64::encode(compressed_assets))
        .send(api)
        .await
        .context("unable to upload problem")?;

    Ok(())
}
