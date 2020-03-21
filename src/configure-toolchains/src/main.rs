#![feature(drain_filter)]

mod config;
mod debootstrap;
mod trace;

use anyhow::Context;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Clone)]
struct Options {
    /// Template files dir
    tpls_dir: PathBuf,
    /// Out dir
    out: PathBuf,
    /// Trace log
    #[structopt(long, short = "t")]
    trace: Option<PathBuf>,
    /// Blacklist: listed toolchains will not be processed
    #[structopt(long)]
    skip: Vec<String>,
    /// Whitelist: only listed toolchains will be processed (overrides `skip`)
    #[structopt(long)]
    only: Vec<String>,
    /// (strategy=trace) Do not treat symlinks like regular files
    #[structopt(long)]
    copy_symlinks: bool,

    /// (strategy=trace) Allow copying directories
    #[structopt(long)]
    copy_dirs: bool,

    /// Instead of populating target dir with files, log all actions to file
    #[structopt(long)]
    dry_run: Option<PathBuf>,
    /// Allowed strategies.
    /// First strategy will have the biggest priority, and so on.
    #[structopt(long)]
    strategies: Vec<String>,
    /// (strategy=trace) Use lxtrace
    #[structopt(long)]
    lxtrace: bool,
}

#[derive(Clone)]
struct ToolchainSpec {
    dir: PathBuf,
    name: String,
    cfg: config::ToolchainConfig,
}
mod tpl_info_impls {
    use super::*;
    use std::{cmp::*, hash::*};

    impl Hash for ToolchainSpec {
        fn hash<H: Hasher>(&self, hasher: &mut H) {
            self.name.hash(hasher);
        }
    }

    impl PartialEq for ToolchainSpec {
        fn eq(&self, that: &ToolchainSpec) -> bool {
            self.name == that.name
        }
    }

    impl Eq for ToolchainSpec {}
}

fn list_templates(dir: &Path) -> anyhow::Result<Vec<ToolchainSpec>> {
    let content = std::fs::read_dir(dir).context("failed to read toolchain templates dir")?;
    let mut out = Vec::new();
    for item in content {
        let item = item.context("failed to stat toolchain template dir")?;
        let cfg = item.path().join("config.yaml");
        let cfg = std::fs::read_to_string(&cfg)
            .with_context(|| format!("failed to open manifest {}", cfg.display()))?;
        let cfg = serde_yaml::from_str(&cfg).context("failed to parse manifest")?;
        let name = item
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .context("toolchain name is not utf8")?
            .to_string();
        out.push(ToolchainSpec {
            dir: item.path(),
            name,
            cfg,
        });
    }
    Ok(out)
}

fn select_templates(tpls: &[ToolchainSpec], opt: &Options) -> anyhow::Result<Vec<ToolchainSpec>> {
    let filter: Box<dyn FnMut(&ToolchainSpec) -> bool> = if !opt.only.is_empty() {
        Box::new(|tpl| opt.only.contains(&tpl.name) || tpl.cfg.auto)
    } else if !opt.skip.is_empty() {
        Box::new(|tpl| !opt.skip.contains(&tpl.name))
    } else {
        Box::new(|_tpl| true)
    };
    let roots: Vec<_> = tpls.iter().cloned().filter(filter).collect();
    let mut q = std::collections::HashSet::new();
    let mut used = std::collections::HashSet::new();
    q.extend(roots.into_iter());

    while let Some(head) = q.iter().next() {
        let tpl = head.clone();
        q.remove(&tpl);
        used.insert(tpl.clone());
        for dep_name in &tpl.cfg.depends {
            let dep = tpls
                .iter()
                .find(|d| d.name.as_str() == dep_name)
                .context("dependency not found")?
                .clone();
            if !used.contains(&dep) {
                q.insert(dep);
            }
        }
    }
    Ok(used.into_iter().collect())
}

trait Resolver {
    fn strategy_name(&self) -> &'static str;
    fn strategy(&self) -> config::Strategy;
    fn visit_spec(
        &mut self,
        spec: &ToolchainSpec,
        log: Option<&mut dyn std::io::Write>,
    ) -> anyhow::Result<()>;

    fn finish(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

fn make_resolvers(opt: &Options) -> anyhow::Result<Vec<Box<dyn Resolver>>> {
    let mut out = vec![
        Box::new(trace::TraceResolver::new(opt)) as Box<dyn Resolver>,
        Box::new(debootstrap::DebootstrapResolver::new(opt)) as Box<dyn Resolver>,
    ];
    if !opt.strategies.is_empty() {
        out.drain_filter(|resolver| {
            opt.strategies
                .iter()
                .any(|allowed_strategy| allowed_strategy == resolver.strategy_name())
        });
        out.sort_unstable_by_key(|resolver| {
            opt.strategies
                .iter()
                .position(|strat| strat == resolver.strategy_name())
                .expect("disallowed resolvers were filtered out");
        })
    }
    Ok(out)
}

fn main() -> anyhow::Result<()> {
    let opt: Options = Options::from_args();
    let mut log_file = match &opt.trace {
        Some(path) => {
            let wr = std::fs::File::create(path).context("failed to open trace log")?;
            let wr = std::io::BufWriter::new(wr);

            Some(wr)
        }
        None => None,
    };

    let mut resolvers = make_resolvers(&opt).context("failed to create resolvers")?;

    let specs = list_templates(&opt.tpls_dir)?;
    let specs = select_templates(&specs, &opt)?;
    for spec in specs {
        println!("------ processing {} ------", &spec.name);
        let mut processed = false;
        for resolver in &mut resolvers {
            if !spec
                .cfg
                .strategies
                .iter()
                .any(|s| s == &resolver.strategy())
            {
                continue;
            }
            println!("Using strategy: {}", resolver.strategy_name());
            processed = true;
            resolver.visit_spec(
                &spec,
                log_file.as_mut().map(|x| x as &mut dyn std::io::Write),
            )?;
            break;
        }
        if !processed {
            eprintln!(
                "Toolchain can not be processed using available strategies ({:?})",
                &opt.strategies
            );
        }
    }

    for resolver in &mut resolvers {
        resolver.finish()?;
    }

    Ok(())
}
