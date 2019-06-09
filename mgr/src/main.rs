use std::process::exit;
use structopt::StructOpt;

#[derive(StructOpt)]
struct SubmissionOpt {
    action: String,
    _filter: String,
}

#[derive(StructOpt)]
enum SubcommandOpt {
    #[structopt(name = "submits")]
    Submission(SubmissionOpt),
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(long = "token", short = "t")]
    token: Option<String>,
    #[structopt(long = "endpoint", short = "e")]
    endpoint: Option<String>,
    #[structopt(subcommand)]
    sub: SubcommandOpt,
}

struct GlobalOptions {
    token: String,
    endpoint: String,
}

impl GlobalOptions {
    fn from_opt(opt: &Opt) -> GlobalOptions {
        let token;
        if let Some(t) = &opt.token {
            token = t.to_string();
        } else {
            token = match std::env::var("JJS_TOKEN") {
                Ok(tok) => tok,
                Err(_) => {
                    eprintln!("Auth not specified");
                    exit(1);
                }
            }
        }

        let endpoint;
        if let Some(ep) = &opt.endpoint {
            endpoint = ep.to_string();
        } else {
            endpoint = std::env::var("JJS_ENDPOINT").unwrap_or("http://localhost:1779".to_string());
        }
        GlobalOptions { token, endpoint }
    }
}

fn manage_submissions(gl_opt: &GlobalOptions, opt: &SubmissionOpt) {
    let client = frontend_api::Client::new(gl_opt.endpoint.clone(), gl_opt.token.clone());
    // at first, load submissions from DB
    // TODO optimizations
    let subm_list_query = frontend_api::SubmissionsListParams {
        limit: u32::max_value(),
    };
    let submissions = client
        .submissions_list(&subm_list_query)
        .unwrap()
        .expect("reqwest rejected");
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
                    status: None,
                    id,
                };
                client
                    .submissions_set_info(&query)
                    .unwrap()
                    .expect("reqwest rejected");
            }
        }

        _ => {
            eprintln!("unknown submissions command: {}", opt.action);
            exit(1);
        }
    }
}

fn main() {
    let opt: Opt = Opt::from_args();
    let gl_opt = GlobalOptions::from_opt(&opt);
    match opt.sub {
        SubcommandOpt::Submission(ref subm) => manage_submissions(&gl_opt, subm),
    }
}
