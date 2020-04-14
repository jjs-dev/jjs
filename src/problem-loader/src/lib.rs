use anyhow::Context;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

struct ProblemItem {
    problem: pom::Problem,
    path: PathBuf,
}

impl ProblemItem {
    fn borrow(&self) -> (&pom::Problem, &Path) {
        (&self.problem, &self.path)
    }
}

struct ProblemsData {
    data: HashMap<String, ProblemItem>,
}

#[derive(Clone)]
pub struct Loader(Arc<ProblemsData>);

fn load_problem_from_dir(dir: &Path) -> anyhow::Result<pom::Problem> {
    let problem_manifest_path = dir.join("manifest.json");

    let problem_manifest =
        std::fs::read(&problem_manifest_path).context("failed to read problem manifest")?;

    let problem_manifest: pom::Problem =
        serde_json::from_slice(&problem_manifest).context("invalid manifest")?;
    Ok(problem_manifest)
}

impl Loader {
    pub fn list(&self) -> impl Iterator<Item = (&pom::Problem, &Path)> {
        self.0.data.values().map(ProblemItem::borrow)
    }

    pub fn find(&self, name: &str) -> Option<(&pom::Problem, &Path)> {
        self.0.data.get(name).map(|item| item.borrow())
    }

    pub fn from_dir(dir: &Path) -> anyhow::Result<Loader> {
        let dir_items = std::fs::read_dir(dir).context("failed to open problems dir")?;
        let mut data = HashMap::new();
        for dir_item in dir_items {
            let dir_item = dir_item.context("failed to stat problem dir")?;
            let path = dir_item.path();
            let problem = load_problem_from_dir(&path)
                .with_context(|| format!("failed to load problem from {}", path.display()))?;
            let item = ProblemItem { problem, path };
            data.insert(item.problem.name.clone(), item);
        }
        Ok(Loader(Arc::new(ProblemsData { data })))
    }

    pub fn load_from_data_dir(jjs_data_dir: &Path) -> anyhow::Result<Loader> {
        Loader::from_dir(&jjs_data_dir.join("var/problems"))
    }

    pub fn empty() -> Loader {
        Loader(Arc::new(ProblemsData {
            data: HashMap::new(),
        }))
    }
}
