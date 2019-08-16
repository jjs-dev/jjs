mod contests;
mod queries;
mod submissions;
mod submit;

use frontend_api::*;
use slog::{o, Drain, Logger};
use std::process::exit;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(long = "dump-completion")]
    dump_completion: bool,
    #[structopt(subcommand)]
    sub: SubOpt,
}

#[derive(StructOpt)]
enum SubOpt {
    #[structopt(name = "submit")]
    Submit(submit::Opt),
    #[structopt(name = "runs")]
    ManageSubmissions(submissions::Opt),
    #[structopt(name = "contests")]
    Contests(contests::Opt),
}

pub struct CommonParams {
    client: Client,
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

fn main() {
    if std::env::var("COMPLETION").is_ok() {
        gen_completion();
        exit(0);
    }

    let opt: Opt = Opt::from_args();

    // TODO works pretty bad
    if opt.dump_completion {
        gen_completion();
        exit(0);
    }

    let drain =
        slog_term::CompactFormat::new(slog_term::TermDecorator::new().stderr().build()).build();

    let logger = slog_envlogger::new(drain);
    let logger = std::sync::Mutex::new(logger);
    let logger = Logger::root(logger.fuse(), o!()).into_erased();
    let _guard = slog_scope::set_global_logger(logger.clone());
    slog_stdlog::init().unwrap();

    let client = Client::from_env();

    let common = CommonParams { client, logger };

    let data = match opt.sub {
        SubOpt::Submit(sopt) => submit::exec(sopt, &common),
        SubOpt::ManageSubmissions(sopt) => submissions::exec(sopt, &common),
        SubOpt::Contests(sopt) => contests::exec(sopt, &common),
    };

    let data = serde_json::to_string_pretty(&data).unwrap();

    println!("{}", data);
}
