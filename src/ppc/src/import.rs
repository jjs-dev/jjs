mod problem_importer;
mod template;
mod valuer_cfg;

use anyhow::{bail, Context as _};
use problem_importer::Importer;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

fn import_one_problem(src: &Path, dest: &Path) -> anyhow::Result<()> {
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
    };

    importer.run()?;

    let manifest_path = dest.join("problem.toml");
    let manifest_toml =
        toml::Value::try_from(importer.problem_cfg.clone()).expect("serialize ppc config");
    let manifest_data = toml::ser::to_string_pretty(&manifest_toml).unwrap_or_else(|err| {
        panic!(
            "stringify ppc config: {}\n\nraw config: {:#?}",
            err, &importer.problem_cfg
        )
    });
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

pub fn exec(args: crate::args::ImportArgs) -> anyhow::Result<()> {
    if args.force {
        std::fs::remove_dir_all(&args.out_path).context("remove out dir")?;
        std::fs::create_dir(&args.out_path).context("recreate out dir")?;
    } else {
        crate::check_dir(&PathBuf::from(&args.out_path), false /* TODO */);
    }

    let src = Path::new(&args.in_path);
    let dest = Path::new(&args.out_path);
    let kind = detect_import_kind(src).context("failed to detect import operation kind")?;
    match kind {
        ImportKind::Problem => import_one_problem(src, dest)?,
        ImportKind::Contest => {
            println!("Importing contest");
            println!("Importing problems");
            let items = src.join("problems").read_dir()?;
            for item in items {
                let item = item?;
                let problem_name = item.file_name();
                println!("--- Importing problem {} ---", problem_name.to_string_lossy());
                let problem_dir = item.path();
                let target_dir = dest.join("problems").join(&problem_name);
                std::fs::create_dir_all(&target_dir)?;
                import_one_problem(&problem_dir, &target_dir)?;
            }
        }
    }
    Ok(())
}
