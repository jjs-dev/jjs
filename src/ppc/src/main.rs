#![feature(is_sorted)]
#![allow(clippy::needless_lifetimes)]

mod command;
mod compile;
mod import;
mod manifest;

mod args {
    use std::path::PathBuf;
    use structopt::StructOpt;

    #[derive(Debug, StructOpt)]
    pub struct CompileArgs {
        /// Path to problem package root
        #[structopt(long = "pkg", short = "P")]
        pub pkg_path: Vec<PathBuf>,
        /// Output path
        #[structopt(long = "out", short = "O")]
        pub out_path: Vec<PathBuf>,
        /// Upload compiled packages
        #[structopt(long, short = "u")]
        pub upload: bool,
        /// Rewrite dir
        #[structopt(long, short = "F")]
        pub force: bool,
    }

    #[derive(StructOpt)]
    pub struct ImportArgs {
        /// Path to package being imported
        #[structopt(long = "in", short = "I")]
        pub in_path: String,
        /// Out path (will contain ppc package)
        #[structopt(long = "out", short = "O")]
        pub out_path: String,
        /// Rewrite dir
        #[structopt(long, short = "F")]
        pub force: bool,
        /// Write contest config to jjs data_dir.
        /// This option can only be used when importing contest
        #[structopt(long, short = "C")]
        pub update_cfg: bool,
        /// Imported contest name
        /// This option can only be used when importing contest
        #[structopt(long, short = "N")]
        pub contest_name: Option<String>,
        /// Build imported problems and install them to jjs data_dir
        #[structopt(long, short = "B")]
        pub build: bool,
    }

    #[derive(StructOpt)]
    #[structopt(author, about)]
    pub enum Args {
        Compile(CompileArgs),
        Import(ImportArgs),
    }
}

use anyhow::Context as _;
use args::Args;
use std::{future::Future, path::Path, process::Stdio};

fn check_dir(path: &Path, allow_nonempty: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("error: path {} not exists", path.display());
    }
    if !path.is_dir() {
        anyhow::bail!("error: path {} is not directory", path.display());
    }
    if !allow_nonempty && path.read_dir().unwrap().next().is_some() {
        anyhow::bail!("error: dir {} is not empty", path.display());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn tune_linux() -> anyhow::Result<()> {
    let mut current_limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        if libc::prlimit(0, libc::RLIMIT_STACK, std::ptr::null(), &mut current_limit) != 0 {
            anyhow::bail!("get current RLIMIT_STACK");
        }
    }
    let new_limit = libc::rlimit {
        rlim_cur: current_limit.rlim_max,
        rlim_max: current_limit.rlim_max,
    };
    unsafe {
        if libc::prlimit(0, libc::RLIMIT_STACK, &new_limit, std::ptr::null_mut()) != 0 {
            anyhow::bail!("update RLIMIT_STACK");
        }
    }

    Ok(())
}

fn tune_resourece_limits() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    tune_linux()?;

    Ok(())
}

/// The most "interesting" functionality of ppc is contained in services,
/// following request-response pattern. It will simplify further daemon mode.
/// `Services` struct manages all there services.
/// # Drop
/// This struct must be dropped using `close` method for correctness.
struct Services {
    pub compiler: compile::CompilerServiceClient,
}

impl Services {
    async fn new() -> anyhow::Result<Self> {
        let compiler = compile::CompilerService::start().await?;
        Ok(Self { compiler })
    }

    async fn shutdown(mut self) -> anyhow::Result<()> {
        self.compiler.close();
        loop {
            let state = self.compiler.state();
            if !state.service_running && state.in_flight_requests == 0 {
                break;
            }
            self.compiler.state_changed().await?;
        }
        std::mem::forget(self);
        Ok(())
    }
}

impl Drop for Services {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // double panic is not cool
            return;
        }
        panic!("ppc::Services must be consumed using shutdown()")
    }
}

fn run_in_background(fut: impl Future<Output = anyhow::Result<()>> + Send + 'static) {
    tokio::task::spawn(async move {
        if let Err(err) = fut.await {
            eprintln!("Error: {:#}", err);
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use structopt::StructOpt;
    tune_resourece_limits()?;
    let args = Args::from_args();
    let services = Services::new().await?;
    let res = process_args(args, &services).await;
    services.shutdown().await.context("finalization error")?;
    res?;
    Ok(())
}

async fn process_args(args: Args, services: &Services) -> anyhow::Result<()> {
    match args {
        Args::Compile(compile_args) => {
            if compile_args.out_path.len() != compile_args.pkg_path.len() {
                anyhow::bail!("count(--pkg) != count(--out)");
            }
            for (out_path, pkg_path) in compile_args.out_path.iter().zip(&compile_args.pkg_path) {
                let args = compile::CompileSingleProblemArgs {
                    out_path: out_path.clone(),
                    pkg_path: pkg_path.clone(),
                    force: compile_args.force,
                };
                crate::run_in_background(services.compiler.exec(args));
            }
        }
        Args::Import(import_args) => {
            import::exec(&services, import_args).await?;
        }
    }
    Ok(())
}
