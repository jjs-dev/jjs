mod contest_import;
mod problem_importer;
mod template;
mod valuer_cfg;

use anyhow::{bail, Context as _};
use problem_importer::Importer;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

fn import_one_problem(src: &Path, dest: &Path, build: bool, force: bool) -> anyhow::Result<()> {
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
    if build {
        println!("Building problem {}", &importer.problem_cfg.name);
        let problems_dir: PathBuf = std::env::var("JJS_DATA")?.into();
        let out_path = problems_dir
            .join("var/problems")
            .join(&importer.problem_cfg.name);
        if force {
            std::fs::remove_dir_all(&out_path).ok();
        }
        std::fs::create_dir(&out_path)?;
        crate::compile_problem(crate::args::CompileArgs {
            pkg_path: dest.to_path_buf(),
            out_path,
            force,
            verbose: true,
        });
    }
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
        std::fs::remove_dir_all(&args.out_path).ok();
        std::fs::create_dir(&args.out_path).context("create out dir")?;
    } else {
        crate::check_dir(&PathBuf::from(&args.out_path), false /* TODO */);
    }

    let src = Path::new(&args.in_path);
    let dest = Path::new(&args.out_path);
    let kind = detect_import_kind(src).context("failed to detect import operation kind")?;
    match kind {
        ImportKind::Problem => import_one_problem(src, dest, args.build, args.force)?,
        ImportKind::Contest => {
            println!("Importing contest");
            println!("Importing problems");
            let items = src.join("problems").read_dir()?;
            for item in items {
                let item = item?;
                let problem_name = item.file_name();
                println!(
                    "--- Importing problem {} ---",
                    problem_name.to_string_lossy()
                );
                let problem_dir = item.path();
                let target_dir = dest.join("problems").join(&problem_name);
                std::fs::create_dir_all(&target_dir)?;
                import_one_problem(&problem_dir, &target_dir, args.build, args.force)?;
            }
            if args.update_cfg {
                let contest_name = args
                    .contest_name
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("missing --contest-name"))?;
                let contest_config = contest_import::import(&src.join("contest.xml"), contest_name)
                    .context("import contest config")?;
                let jjs_data_dir = std::env::var("JJS_DATA").context("JJS_DATA missing")?;
                let path = PathBuf::from(jjs_data_dir)
                    .join("etc/objects/contests")
                    .join(format!("{}.yaml", contest_name));
                if path.exists() && !args.force {
                    anyhow::bail!("path {} already exists", path.display());
                }
                let contest_config = serde_yaml::to_string(&contest_config)?;
                std::fs::write(path, contest_config)?;
            }
        }
    }
    Ok(())
}
