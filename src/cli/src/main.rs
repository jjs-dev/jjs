mod api_version;
mod contests;
mod submissions;
mod submit;

use client::ApiClient;
use slog::{o, Drain, Logger};
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
    logger: Logger,
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

    let opt: Opt = Opt::from_args();

    let drain =
        slog_term::CompactFormat::new(slog_term::TermDecorator::new().stderr().build()).build();

    let logger = slog_envlogger::new(drain);
    let logger = std::sync::Mutex::new(logger);
    let logger = Logger::root(logger.fuse(), o!()).into_erased();
    let _guard = slog_scope::set_global_logger(logger.clone());
    slog_stdlog::init().unwrap();

    let client = client::connect();

    let common = CommonParams { client, logger };

    let data = match opt.sub {
        SubOpt::Submit(sopt) => submit::exec(sopt, &common).await,
        SubOpt::ManageSubmissions(sopt) => submissions::exec(sopt, &common).await,
        SubOpt::Contests => contests::exec(&common).await,
        SubOpt::ApiVersion => api_version::exec(&common).await,
    };

    let data = serde_json::to_string_pretty(&data).unwrap();

    println!("{}", data);
}
