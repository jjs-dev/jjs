use graphql_client::GraphQLQuery;
use serde_json::Value;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opt {
    /// Show detailed information
    #[structopt(long)]
    detailed: bool,
}

pub fn exec(opt: Opt, common: &super::CommonParams) -> Value {
    let vars = crate::queries::list_contests::Variables {
        detailed: opt.detailed,
    };
    let res = common
        .client
        .query::<_, crate::queries::list_contests::ResponseData>(
            &crate::queries::ListContests::build_query(vars),
        )
        .expect("network error")
        .into_result();
    match res {
        Ok(data) => serde_json::to_value(data).unwrap(),
        Err(e) => {
            eprintln!("error: {}", e[0]);
            Value::Null
        }
    }
}
