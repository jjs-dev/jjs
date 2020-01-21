use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opts {
    #[structopt(long)]
    data_dir: Option<PathBuf>,
    #[structopt(long)]
    install_dir: PathBuf,
    /// Connection string for database to setup
    ///
    /// If not provided, db setup will be skipped
    #[structopt(long)]
    db_url: Option<String>,
    /// Drop db if it already exists
    #[structopt(long)]
    drop_db: bool,
    #[structopt(long)]
    symlink_config: bool,
    #[structopt(long)]
    setup_config: bool,
    /// Build sample contest (requires ppc to be available)
    #[structopt(long)]
    sample_contest: bool,
    /// Configure toolchains
    #[structopt(long)]
    toolchains: bool,
    /// Force mode: ignore some errors
    #[structopt(long)]
    force: bool,
    /// Touch file on success
    #[structopt(long)]
    touch: Option<PathBuf>,
}

fn main() {
    let opts: Opts = Opts::from_args();
    util::log::setup();
    util::wait::wait();
    let params = setup::SetupParams {
        data_dir: opts.data_dir,
        install_dir: opts.install_dir,
        db: if let Some(uri) = opts.db_url {
            Some(setup::DatabaseParams {
                uri,
                drop_existing: opts.drop_db,
            })
        } else {
            None
        },
        config: if opts.setup_config {
            Some(setup::ConfigParams {
                symlink: opts.symlink_config,
            })
        } else {
            None
        },
        toolchains: opts.toolchains,
        sample_contest: opts.sample_contest,
        force: opts.force,
    };
    let runner = util::cmd::Runner::new();
    if let Err(e) = setup::setup(&params, &runner) {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }
    runner.exit_if_errors();
    if let Some(touch) = &opts.touch {
        log::info!("Touching {}", touch.display());
        std::fs::File::create(touch).ok();
    }
}
