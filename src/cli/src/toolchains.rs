use anyhow::Context as _;
use client::prelude::Sendable;

#[derive(clap::Clap)]
pub(crate) struct Opt {
    /// Toolchain image to add
    #[clap(long)]
    image: String,
    /// Toolchain name. If not provided, will be inferred from image labels.
    #[clap(long)]
    name: Option<String>,
    /// Pull image before using
    #[clap(long)]
    pull: bool,
}

pub(crate) async fn exec(opt: &Opt, api: &client::ApiClient) -> anyhow::Result<()> {
    if opt.pull {
        println!("pulling the image");
        let mut cmd = tokio::process::Command::new("docker");
        cmd.arg("pull");
        cmd.arg(&opt.image);
        let status = cmd.status().await.context("docker not available")?;
        if !status.success() {
            anyhow::bail!("docker pull failed");
        }
    }
    let toolchain_name = match &opt.name {
        Some(name) => name.clone(),
        None => {
            println!("Inspecting for label value");
            let mut cmd = tokio::process::Command::new("docker");
            cmd.arg("inspect");
            cmd.arg(&opt.image);
            let out = cmd.output().await?;
            eprintln!("{}", String::from_utf8_lossy(&out.stderr));
            if !out.status.success() {
                anyhow::bail!("docker inspect failed");
            }
            let image_description: serde_json::Value =
                serde_json::from_slice(&out.stdout).context("parse error")?;
            image_description
                .pointer("/Config/Labels/io.jjs.toolchain.name")
                .context("label io.jjs.toolchain.name missing")?
                .as_str()
                .context("malformed docker inspect output: label is not a string")?
                .to_string()
        }
    };
    println!("Toolchain name: {}", &toolchain_name);

    client::models::Toolchain::put_toolchain()
        .image(&opt.image)
        .id(&toolchain_name)
        .description("TODO")
        .send(api)
        .await?;

    Ok(())
}
