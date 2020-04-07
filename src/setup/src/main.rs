pub mod profile;

use anyhow::Context as _;
use profile::Profile;
use setup::Component;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tokio::io::AsyncReadExt as _;

#[derive(StructOpt, Copy, Clone)]
enum Subcommand {
    Describe,
    Upgrade,
}

impl Subcommand {
    fn is_upgrade(self) -> bool {
        match self {
            Subcommand::Describe => false,
            Subcommand::Upgrade => true,
        }
    }
}

#[derive(StructOpt)]
struct Opts {
    profile: PathBuf,
    #[structopt(subcommand)]
    action: Subcommand,
}

enum SystemHealth {
    Ok,
    Error,
}

async fn process_component<C: Component>(
    component: C,
    upgrade: bool,
) -> Result<SystemHealth, C::Error> {
    let kind = component.state().await?;
    let name = component.name();
    let health = if matches!(kind, setup::StateKind::Errored) {
        SystemHealth::Error
    } else {
        SystemHealth::Ok
    };
    if upgrade {
        match kind {
            setup::StateKind::Errored => {
                eprintln!(
                    "Skipping {} update, because it's current state is Error",
                    name
                );
            }
            setup::StateKind::UpToDate => {
                println!("Skipping {} update: it is up-to-date", name);
            }
            setup::StateKind::Upgradable => {
                println!("Updating {}", name);
                component.upgrade().await?;
            }
        }
    } else {
        println!("{} state: {} ({})", name, kind, component);
    }
    Ok(health)
}

async fn process_db(profile: &Profile, action: Subcommand) -> anyhow::Result<SystemHealth> {
    let pg_settings = match profile.pg.as_ref() {
        Some(pg) => setup::db::ConnectionSettings {
            conn_string: pg.conn_string.clone(),
            db_name: pg.db_name.clone(),
        },
        None => return Ok(SystemHealth::Ok),
    };

    let db_cx = setup::db::DbContext {
        settings: &pg_settings,
        install_dir: &profile.install_dir,
    };

    let db = setup::db::analyze(db_cx)
        .await
        .context("failed to analyze db state")?;
    process_component(db, action.is_upgrade())
        .await
        .context("process db")
}

async fn process_data(profile: &Profile, action: Subcommand) -> anyhow::Result<SystemHealth> {
    let data_dir = match &profile.data_dir {
        Some(dd) => dd,
        None => return Ok(SystemHealth::Ok),
    };
    let cx = setup::data::Context { data_dir };
    let data_dir_layout = setup::data::analyze(cx)
        .await
        .context("analyze data dir layout")?;
    process_component(data_dir_layout, action.is_upgrade())
        .await
        .context("process data dir layout")
}

async fn process_configs(profile: &Profile, action: Subcommand) -> anyhow::Result<SystemHealth> {
    let data_dir = match &profile.data_dir {
        Some(dd) => dd,
        None => return Ok(SystemHealth::Ok),
    };
    if !profile.configs {
        return Ok(SystemHealth::Ok);
    }
    let cx = setup::config::Context {
        data_dir: &data_dir,
        install_dir: &profile.install_dir,
    };
    let configs = setup::config::analyze(cx)
        .await
        .context("analyze configs")?;
    process_component(configs, action.is_upgrade())
        .await
        .context("process configs")
}

async fn process_toolchains(profile: &Profile, action: Subcommand) -> anyhow::Result<SystemHealth> {
    let data_dir = match &profile.data_dir {
        Some(dd) => dd,
        None => return Ok(SystemHealth::Ok),
    };
    let tcs_profile = match &profile.toolchains {
        Some(tcs) => tcs,
        None => return Ok(SystemHealth::Ok),
    };
    let mut custom_argv = Vec::new();
    for arg in &tcs_profile.additional_args {
        custom_argv.push(std::ffi::OsStr::new(arg));
    }
    let mut strategies = Vec::new();
    for strat in &tcs_profile.strategies {
        strategies.push(strat.as_str());
    }
    let filter = |toolchain: &str| {
        if tcs_profile
            .blacklist
            .iter()
            .any(|blacklisted| blacklisted == toolchain)
        {
            return false;
        }
        if tcs_profile.whitelist.is_empty() {
            true
        } else {
            tcs_profile
                .whitelist
                .iter()
                .any(|whitelisted| whitelisted == toolchain)
        }
    };
    let cx = setup::toolchains::Context {
        data_dir,
        install_dir: &profile.install_dir,
        custom_argv: &custom_argv,
        strategies: &strategies,
        filter: &filter,
    };
    let toolchains = setup::toolchains::analyze(cx)
        .await
        .context("analyze toolchains")?;
    process_component(toolchains, action.is_upgrade())
        .await
        .context("process toolchains")
}

async fn process_problems(profile: &Profile, action: Subcommand) -> anyhow::Result<SystemHealth> {
    let data_dir = match &profile.data_dir {
        Some(dd) => dd,
        None => return Ok(SystemHealth::Ok),
    };
    let prof = match &profile.problems {
        Some(p) => p,
        None => return Ok(SystemHealth::Ok),
    };
    let mut compile_paths = Vec::new();
    let mut archive_paths = Vec::new();
    for source in &prof.compile.sources {
        let profile::Source::Path { path } = &source;
        compile_paths.push(path.as_path());
    }
    for archive in &prof.archive.sources {
        let profile::Source::Path { path } = &archive;
        archive_paths.push(path.as_path());
    }
    let cx = setup::problems::Context {
        data_dir,
        install_dir: &profile.install_dir,
        compile_paths: &compile_paths,
        archive_paths: &archive_paths,
    };
    let problems = setup::problems::analyze(cx)
        .await
        .context("analyze problems")?;
    process_component(problems, action.is_upgrade())
        .await
        .context("process problems")
}

async fn load_profile(path: &Path) -> anyhow::Result<Profile> {
    let profile_data = if path == Path::new("-") {
        let mut buf = String::new();
        tokio::io::stdin()
            .read_to_string(&mut buf)
            .await
            .context("read profile data from stdin")?;
        buf
    } else {
        tokio::fs::read_to_string(path)
            .await
            .context("read profile data from given file")?
    };
    let profile = serde_yaml::from_str(&profile_data).context("parse profile")?;
    Ok(profile)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    util::log::setup();
    util::wait::wait();
    let opts: Opts = Opts::from_args();
    let profile = load_profile(&opts.profile).await.context("load profile")?;
    std::env::set_var("JJS_PATH", &profile.install_dir);
    if let Some(data_dir) = &profile.data_dir {
        std::env::set_var("JJS_DATA", data_dir);
    }
    let mut healthes = Vec::new();
    healthes.push(process_db(&profile, opts.action).await?);
    healthes.push(process_data(&profile, opts.action).await?);
    healthes.push(process_configs(&profile, opts.action).await?);
    healthes.push(process_toolchains(&profile, opts.action).await?);
    healthes.push(process_problems(&profile, opts.action).await?);

    for h in healthes {
        if matches!(h, SystemHealth::Error) {
            return Err(anyhow::anyhow!("some components are errored"));
        }
    }
    println!("System is healthy");
    Ok(())
}
