use graphql_client::GraphQLQuery;
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

pub fn exec(opt: Opt, params: &super::CommonParams) -> Value {
    // at first, load submissions from DB
    // TODO optimizations
    let vars = crate::queries::list_runs::Variables { detailed: true };
    // FIXME:                                                   ^
    //                                       should be false here
    //                                see https://github.com/graphql-rust/graphql-client/issues/250
    let query = crate::queries::ListRuns::build_query(vars);
    let submissions = params
        .client
        .query::<_, crate::queries::list_runs::ResponseData>(&query)
        .expect("transport error")
        .into_result()
        .expect("api error")
        .submissions;
    match opt.action.as_str() {
        "view" => serde_json::to_value(&submissions).unwrap(),
        "remove" => {
            let mut result = vec![];
            for sbm in &submissions {
                let id = sbm.id;
                //println!("deleting submission {}", id);
                result.push(id);
                let vars = crate::queries::remove_run::Variables { run_id: id };
                params
                    .client
                    .query::<_, crate::queries::remove_run::ResponseData>(
                        &crate::queries::RemoveRun::build_query(vars),
                    )
                    .unwrap()
                    .into_result()
                    .map(std::mem::drop)
                    .unwrap_or_else(|err| {
                        error!(
                            params.logger,
                            "api error when deleting submission {}: {}", id, err[0]
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
                let vars = crate::queries::rejudge_run::Variables { run_id: id };
                let query = crate::queries::RejudgeRun::build_query(vars);
                let res = params
                    .client
                    .query::<_, crate::queries::rejudge_run::ResponseData>(&query)
                    .unwrap()
                    .into_result();
                if let Err(e) = res {
                    error!(
                        params.logger,
                        "When rejudging run {}: api error: {}", id, e[0]
                    );
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
