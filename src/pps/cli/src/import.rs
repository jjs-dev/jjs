use anyhow::Context as _;
use pps_api::import_problem::{PropertyName, Update};
use std::path::{Path, PathBuf};

#[derive(clap::Clap, Debug)]
pub struct ImportArgs {
    /// Path to package being imported
    #[clap(long = "in", short = 'I')]
    pub in_path: PathBuf,
    /// Out path (will contain ppc package)
    #[clap(long = "out", short = 'O')]
    pub out_path: PathBuf,
    /// Rewrite dir
    #[clap(long, short = 'F')]
    pub force: bool,
    /// Imported contest name
    /// This option can only be used when importing contest
    #[clap(long, short = 'N')]
    pub contest_name: Option<String>,
}

async fn import_one_problem(
    client: &mut rpc::Client,
    src: &Path,
    dest: &Path,
    force: bool,
) -> anyhow::Result<()> {
    let import_req = pps_api::import_problem::Request {
        src_path: src.to_path_buf(),
        out_path: dest.to_path_buf(),
        force,
    };
    let (tx, mut import) = client.start::<pps_api::ImportProblem>().await?;
    tx.send(import_req).await?;
    while let Some(update) = import.next_event().await? {
        match update {
            Update::Property {
                property_name,
                property_value,
            } => match property_name {
                PropertyName::TimeLimit => println!("Time limit: {} ms", property_value),
                PropertyName::MemoryLimit => {
                    let ml = property_value.parse::<u64>()?;
                    println!("Memory limit: {} bytes ({} MiBs)", ml, ml / (1 << 20));
                }
                PropertyName::InputPathPattern => {
                    println!("Test input file path pattern: {}", property_value)
                }
                PropertyName::OutputPathPattern => {
                    println!("Test output file path pattern: {}", property_value)
                }
                PropertyName::ProblemTitle => println!("Problem title: {}", property_value),
            },
            Update::Warning(warning) => eprintln!("warning: {}", warning),
            Update::ImportChecker => println!("Importing checker"),
            Update::ImportTests => println!("Importing tests"),
            Update::ImportTestsDone { count } => println!("{} tests imported", count),
            Update::ImportSolutions => println!("Importing solutions"),
            Update::ImportSolution(solution) => println!("Importing solution '{}'", solution),
            Update::ImportValuerConfig => println!("Importing valuer config"),
            Update::DefaultValuerConfig => println!("Defaulting valuer config"),
        }
    }
    import.finish().await?.0.context("build failure")?;

    println!("Import successful");

    Ok(())
}

#[tracing::instrument(skip(client, args))]
pub(crate) async fn exec(client: &mut rpc::Client, args: ImportArgs) -> anyhow::Result<()> {
    if args.force {
        std::fs::remove_dir_all(&args.out_path).ok();
        std::fs::create_dir(&args.out_path).context("create out dir")?;
    } else {
        crate::check_dir(&PathBuf::from(&args.out_path), false /* TODO */)?;
    }

    let src = &args.in_path;
    let dest = &args.out_path;

    import_one_problem(client, src, dest, args.force).await?;

    // TODO support importing contests

    Ok(())
}
