use client::prelude::Sendable;
/// Wait until apiserver responds
#[derive(clap::Clap)]
pub(crate) struct Opt {
    /// Max waiting timeout in seconds
    #[clap(long, default_value = "45")]
    max_timeout: u32,
    /// Time between attempts
    #[clap(long, default_value = "5")]
    interval: u32,
}

pub(crate) async fn exec(opt: &Opt, api: &client::ApiClient) -> anyhow::Result<()> {
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(opt.max_timeout.into());
    loop {
        if client::models::ApiVersion::api_version()
            .send(api)
            .await
            .is_ok()
        {
            println!("\nSuccess");
            break;
        }
        print!(".");
        tokio::io::AsyncWriteExt::flush(&mut tokio::io::stdout()).await?;
        if std::time::Instant::now() > deadline {
            anyhow::bail!("Deadline exceeded");
        }
        tokio::time::delay_for(std::time::Duration::from_secs(opt.interval.into())).await;
    }
    Ok(())
}
