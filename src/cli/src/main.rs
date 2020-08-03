mod api_version;
mod completion;
mod login;
mod problems;
mod runs;
mod submit;
mod toolchains;
mod wait;

use clap::Clap;

/// Command-line client for JJS
#[derive(Clap)]
#[clap(author, about)]
struct Opt {
    #[clap(subcommand)]
    sub: SubOpt,
}

#[derive(Clap)]
enum SubOpt {
    Submit(submit::Opt),
    ManageRuns(runs::Opt),
    Login(login::Opt),
    Toolchains(toolchains::Opt),
    Wait(wait::Opt),
    Problems(problems::Opt),
    Completion(completion::Opt),
    ApiVersion,
}

#[tokio::main]
async fn main() {
    util::log::setup();
    if let Err(err) = real_main().await {
        eprintln!("Error: {:#}", err);
        std::process::exit(1);
    }
}

async fn real_main() -> anyhow::Result<()> {
    let opt: Opt = Opt::parse();

    if let SubOpt::Login(opt) = &opt.sub {
        login::exec(opt).await?;
        return Ok(());
    }

    let client = client::infer().await?;

    match opt.sub {
        SubOpt::Submit(sopt) => submit::exec(sopt, &client).await?,
        SubOpt::ManageRuns(sopt) => runs::exec(sopt, &client).await?,
        SubOpt::ApiVersion => api_version::exec(&client).await?,
        SubOpt::Toolchains(sopt) => toolchains::exec(&sopt, &client).await?,
        SubOpt::Wait(sopt) => wait::exec(&sopt, &client).await?,
        SubOpt::Problems(sopt) => problems::exec(&sopt, &client).await?,
        SubOpt::Completion(sopt) => completion::exec(&sopt)?,
        SubOpt::Login(_) => unreachable!(),
    };
    Ok(())
}
