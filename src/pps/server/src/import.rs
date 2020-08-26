mod problem_importer;
mod template;
mod valuer_cfg;

use anyhow::{bail, Context as _};
use pps_api::{
    import_problem::{Request, Update},
    ImportProblem, SimpleFinish,
};
use problem_importer::Importer;
use std::{collections::HashSet, path::Path};

impl rpc::Handler<ImportProblem> for crate::Service {
    type Error = anyhow::Error;
    type Fut = futures_util::future::BoxFuture<'static, anyhow::Result<()>>;

    fn handle(
        self,
        rx: rpc::UnaryRx<Request>,
        mut tx: rpc::StreamingTx<Update, SimpleFinish>,
    ) -> Self::Fut {
        Box::pin(async move {
            let req = rx.recv().await?;
            let result = execute_import_request(req, &mut tx).await;
            tx.finish(result.into()).await?;
            Ok(())
        })
    }
}

pub(crate) async fn execute_import_request(
    req: Request,
    tx: &mut rpc::StreamingTx<Update, SimpleFinish>,
) -> anyhow::Result<()> {
    match detect_import_kind(&req.src_path)? {
        ImportKind::Problem => (),
        ImportKind::Contest => anyhow::bail!("TODO"),
    }
    import_problem(&req.src_path, &req.out_path, tx).await?;

    Ok(())
}

pub(crate) async fn import_problem(
    src: &Path,
    dest: &Path,
    tx: &mut rpc::StreamingTx<Update, SimpleFinish>,
) -> anyhow::Result<()> {
    let manifest_path = src.join("problem.xml");
    let manifest = std::fs::read_to_string(manifest_path).context("failed read problem.xml")?;
    let doc = roxmltree::Document::parse(&manifest).context("parse error")?;

    let mut importer = Importer {
        src: &src,
        dest: &dest,
        problem_cfg: Default::default(),
        known_generators: HashSet::new(),
        doc: doc.root_element(),
        limits: pom::Limits::default(),
        tx,
    };

    importer.run().await?;

    let manifest_path = dest.join("problem.toml");
    let manifest_toml =
        toml::Value::try_from(importer.problem_cfg.clone()).context("serialize ppc config")?;
    let manifest_data = toml::ser::to_string_pretty(&manifest_toml)
        .with_context(|| format!("stringify ppc config: {:#?}", &importer.problem_cfg))?;
    std::fs::write(manifest_path, manifest_data).expect("write ppc manifest");

    Ok(())
}

enum ImportKind {
    Problem,
    Contest,
}

fn detect_import_kind(path: &Path) -> anyhow::Result<ImportKind> {
    if !path.exists() {
        bail!("path {} does not exists", path.display());
    }

    if path.join("problem.xml").exists() {
        return Ok(ImportKind::Problem);
    }
    if path.join("contest.xml").exists() {
        return Ok(ImportKind::Contest);
    }

    bail!("unknown src")
}
