use structopt::StructOpt;
use frontend_api::{ SubmissionsListParams, SubmissionState, SubmissionsSetInfoParams};
use std::process::exit;

#[derive(StructOpt)]
pub struct Opt {
    action: String,
    _filter: String,
}

pub fn exec(opt: Opt, params: &super::CommonParams) {
    // at first, load submissions from DB
    // TODO optimizations
    let subm_list_query = SubmissionsListParams {
        limit: u32::max_value(),
    };
    let submissions = params.client
        .submissions_list(&subm_list_query)
        .unwrap()
        .expect("request rejected");
    match opt.action.as_str() {
        "view" => {
            println!("submissions: {:?}", &submissions);
        }
        "remove" => {
            println!("deleting {} submissions", submissions.len());
            for sbm in &submissions {
                let id = sbm.id;
                println!("deleting submission {}", id);
                let query = frontend_api::SubmissionsSetInfoParams {
                    delete: true,
                    rejudge: false,
                    status: None,
                    state: None,
                    id,
                };
                params.client
                    .submissions_modify(&query)
                    .unwrap()
                    .expect("request rejected");
            }
        }
        "rejudge" => {
            for sbm in &submissions {
                let id = sbm.id;
                println!("queuing submission {} for rejudge", id);
                let query = SubmissionsSetInfoParams {
                    delete: false,
                    rejudge: false,
                    id,
                    status: None,
                    state: Some(SubmissionState::Queue),
                };
                params.client
                    .submissions_modify(&query)
                    .unwrap()
                    .expect("request rejected");
            }
        }

        _ => {
            eprintln!("unknown submissions command: {}", opt.action);
            exit(1);
        }
    }
}