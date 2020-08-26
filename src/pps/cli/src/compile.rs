use anyhow::Context as _;
use pps_api::compile_problem::Update;
use std::path::PathBuf;

#[derive(Debug, clap::Clap)]
pub struct CompileArgs {
    /// Path to problem package root
    #[clap(long = "pkg", short = "P")]
    pub pkg_path: Vec<PathBuf>,
    /// Output path
    #[clap(long = "out", short = "O")]
    pub out_path: Vec<PathBuf>,
    /// Rewrite dir
    #[clap(long, short = "F")]
    pub force: bool,
}

#[tracing::instrument(skip(client, compile_args))]
pub async fn exec(client: &mut rpc::Client, compile_args: CompileArgs) -> anyhow::Result<()> {
    if compile_args.out_path.len() != compile_args.pkg_path.len() {
        anyhow::bail!("count(--pkg) != count(--out)");
    }
    for (out_path, pkg_path) in compile_args.out_path.iter().zip(&compile_args.pkg_path) {
        let req = pps_api::compile_problem::Request {
            out_path: out_path.clone(),
            problem_path: pkg_path.clone(),
            force: compile_args.force,
        };
        let (tx, mut resp) = client
            .start::<pps_api::CompileProblem>()
            .await
            .context("failed to start RPC call")?;
        tx.send(req).await?;
        let mut notifier = None;
        while let Some(upd) = resp.next_event().await? {
            match upd {
                Update::Warnings(warnings) => {
                    if !warnings.is_empty() {
                        eprintln!("{} warnings", warnings.len());
                        for warn in warnings {
                            eprintln!("- {}", warn);
                        }
                    }
                }
                Update::BuildSolution(solution_name) => {
                    println!("Building solution {}", &solution_name);
                }
                Update::BuildTestgen(testgen_name) => {
                    println!("Building generator {}", testgen_name);
                }
                Update::BuildChecker => {
                    println!("Building checker");
                }
                Update::GenerateTests { count } => {
                    notifier = Some(crate::progress_notifier::Notifier::new(count));
                }
                Update::GenerateTest { test_id } => {
                    notifier
                        .as_mut()
                        .expect("GenerateTest received before GenerateTests")
                        .maybe_notify(test_id);
                }
                Update::CopyValuerConfig => {
                    println!("Valuer config");
                }
            }
        }
        if let Err(err) = resp.finish().await?.0 {
            anyhow::bail!("Failed to build problem: {}", err)
        }
    }
    Ok(())
}
