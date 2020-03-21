use client::Api as _;
use serde_json::Value;
use slog::error;
use std::process::exit;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opt {
    /// Action: view, remove or rejudge
    action: String,
    #[structopt(long = "filter", short = "f", default_value = "true")]
    _filter: String,
}

pub async fn exec(opt: Opt, params: &super::CommonParams) -> Value {
    // at first, load submissions from DB
    // TODO optimizations

    let submissions = params.client.list_runs().await.expect("api error");
    match opt.action.as_str() {
        "view" => serde_json::to_value(&submissions).unwrap(),
        "remove" => {
            let mut result = vec![];
            for sbm in &submissions {
                let id = sbm.id;
                result.push(id);
                params
                    .client
                    .delete_run(id)
                    .await
                    .map(drop)
                    .unwrap_or_else(|err| {
                        error!(
                            params.logger,
                            "api error when deleting submission {}: {:?}", id, err
                        )
                    });
            }
            serde_json::to_value(result).unwrap()
        }
        "rejudge" => {
            let mut result = vec![];
            for sbm in &submissions {
                let id = sbm.id;
                result.push(id);
                let patch = client::models::RunPatch {
                    rejudge: Some(true),
                    score: None,
                };
                let res = params.client.patch_run(id, Some(patch)).await;
                if let Err(e) = res {
                    error!(params.logger, "When rejudging run {}: api error: {}", id, e);
                }
            }
            serde_json::to_value(result).unwrap()
        }

        _ => {
            eprintln!("unknown submissions command: {}", opt.action);
            exit(1);
        }
    }
}
