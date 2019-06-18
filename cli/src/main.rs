mod contests;
mod submissions;
mod submit;

use frontend_api::*;
use std::process::exit;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(long = "token", short = "t", default_value = "dev_root")]
    token: String,
    #[structopt(long = "verbose", short = "v")]
    verbose: bool,
    #[structopt(subcommand)]
    sub: SubOpt,
}

#[derive(StructOpt)]
enum SubOpt {
    #[structopt(name = "submit")]
    Submit(submit::Opt),
    #[structopt(name = "manage-submissions")]
    ManageSubmissions(submissions::Opt),
    #[structopt(name = "contests")]
    Contests(contests::Opt),
}

pub struct CommonParams {
    client: Client,
}

fn gen_completion() {
    let mut clap_app = Opt::clap();
    clap_app.gen_completions_to(
        "jjs-cli",
        structopt::clap::Shell::Bash,
        &mut std::io::stdout(),
    );
}

fn main() {
    if std::env::var("GEN_COMPLETION").is_ok() {
        gen_completion();
        exit(0);
    }
    use sloggers::Build;
    let opt: Opt = Opt::from_args();

    let mut builder = sloggers::terminal::TerminalLoggerBuilder::new();
    if opt.verbose {
        builder.level(sloggers::types::Severity::Debug);
    }

    let logger = builder.build().expect("couldn't setup logger");

    let token = opt.token.clone();

    let client = Client {
        endpoint: "http://localhost:1779".to_string(),
        logger: Some(logger),
        token,
    };

    let common = CommonParams { client };

    match opt.sub {
        SubOpt::Submit(sopt) => submit::exec(sopt, &common),
        SubOpt::ManageSubmissions(sopt) => submissions::exec(sopt, &common),
        SubOpt::Contests(sopt) => contests::exec(sopt, &common),
    }
}
