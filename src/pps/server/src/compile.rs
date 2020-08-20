//! This module implements compiling source package into invoker package
pub(crate) mod build;
mod builder;

use anyhow::Context as _;
use pps_api::{
    compile_problem::{Request, Update},
    SimpleFinish,
};
use std::{path::Path, sync::Arc};

impl rpc::Handler<pps_api::CompileProblem> for crate::Service {
    type Error = anyhow::Error;
    type Fut = futures_util::future::BoxFuture<'static, Result<(), Self::Error>>;

    fn handle(
        self,
        rx: rpc::UnaryRx<Request>,
        mut tx: rpc::StreamingTx<Update, SimpleFinish>,
    ) -> Self::Fut {
        Box::pin(async move {
            let args = rx.recv().await?;
            let response = exec_compile_request(args, self.0, &mut tx).await;
            tx.finish(response.into()).await?;
            Ok(())
        })
    }
}

/// This function actually implements request processing.
/// It's return value is used as response.
pub(crate) async fn exec_compile_request(
    args: Request,
    data: Arc<crate::ServiceState>,
    tx: &mut rpc::StreamingTx<Update, SimpleFinish>,
) -> anyhow::Result<()> {
    if args.force {
        std::fs::remove_dir_all(&args.out_path).ok();
        tokio::fs::create_dir_all(&args.out_path).await?;
    } else {
        crate::check_dir(&args.out_path, false /* TODO */)?;
    }
    let toplevel_manifest = args.problem_path.join("problem.toml");
    let toplevel_manifest = std::fs::read_to_string(toplevel_manifest)?;

    let raw_problem_cfg: crate::manifest::RawProblem =
        toml::from_str(&toplevel_manifest).context("problem.toml parse error")?;
    let (problem_cfg, warnings) = raw_problem_cfg.postprocess()?;

    tx.send_event(Update::Warnings(warnings)).await?;

    let out_dir = args.out_path.canonicalize().context("resolve out dir")?;
    let problem_dir = args
        .problem_path
        .canonicalize()
        .context("resolve problem dir")?;

    let mut builder = builder::ProblemBuilder {
        cfg: &problem_cfg,
        problem_dir: &problem_dir,
        out_dir: &out_dir,
        jtl_dir: &data.jjs_dir,
        build_backend: &build::Pibs {
            jjs_dir: Path::new(&data.jjs_dir),
        },
        tx,
    };
    builder.build().await?;
    Ok(())
}
