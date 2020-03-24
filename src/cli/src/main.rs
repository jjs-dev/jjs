mod api_version;
mod contests;
mod submissions;
mod submit;

use client::ApiClient;
use std::process::exit;
use structopt::StructOpt;

/// Command-line client for JJS
///
/// To get Bash completion, run:
/// COMPLETION=1 <path/to/jjs-cli> > /tmp/compl.sh
/// . /tmp/compl.sh
#[derive(StructOpt)]
#[structopt(author, about)]
struct Opt {
    #[structopt(subcommand)]
    sub: SubOpt,
}

#[derive(StructOpt)]
enum SubOpt {
    Submit(submit::Opt),
    ManageSubmissions(submissions::Opt),
    Contests,
    #[structopt(name = "api-version")]
    ApiVersion,
}

pub struct CommonParams {
    client: ApiClient,
}

fn gen_completion() {
    let mut clap_app = Opt::clap();
    clap_app.gen_completions_to(
        "jjs-cli",
        structopt::clap::Shell::Bash,
        &mut std::io::stdout(),
    );
}

#[tokio::main]
async fn main() {
    if std::env::var("COMPLETION").is_ok() {
        gen_completion();
        exit(0);
    }
    util::log::setup();

    let opt: Opt = Opt::from_args();

    let client = client::connect();

    let common = CommonParams { client };

    let data = match opt.sub {
        SubOpt::Submit(sopt) => submit::exec(sopt, &common).await,
        SubOpt::ManageSubmissions(sopt) => submissions::exec(sopt, &common).await,
        SubOpt::Contests => contests::exec(&common).await,
        SubOpt::ApiVersion => api_version::exec(&common).await,
    };

    let data = serde_json::to_string_pretty(&data).unwrap();

    println!("{}", data);
}
