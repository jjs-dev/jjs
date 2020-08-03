use client::prelude::Sendable as _;

#[derive(clap::Clap)]
pub struct Opt {
    /// Action: view, remove or rejudge
    action: String,
    #[clap(long = "filter", short = "f", default_value = "true")]
    _filter: String,
}

pub async fn exec(opt: Opt, api: &client::ApiClient) -> anyhow::Result<()> {
    // TODO optimizations

    let runs = client::models::Run::list_runs().send(api).await?;
    match opt.action.as_str() {
        "view" => {
            println!("runs: {:?}", runs.object);
            Ok(())
        }
        _ => {
            anyhow::bail!("unknown runs subcommand: {}", opt.action);
        }
    }
}
