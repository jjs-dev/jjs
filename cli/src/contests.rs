use serde_json::Value;
use structopt::StructOpt;
#[derive(StructOpt)]
pub struct Opt {
    contest: Option<String>,
}

pub fn exec(opt: Opt, common: &super::CommonParams) -> Value {
    match opt.contest {
        Some(name) => {
            let info = common
                .client
                .contests_describe(&name)
                .expect("network error")
                .expect("error");
            serde_json::to_value(info).unwrap()
        }
        None => {
            let information = common
                .client
                .contests_list(&())
                .expect("network error")
                .expect("error");
            serde_json::to_value(information).unwrap()
        }
    }
}
