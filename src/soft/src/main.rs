#![feature(process_exitcode_placeholder)]

mod config;
mod dep_collector;

use crate::dep_collector::DepCollector;
use anyhow::{bail, Context};
use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    /// Template files dir
    tpls_dir: PathBuf,
    /// Out dir
    out: PathBuf,
    /// Trace log,
    #[structopt(long, short = "t")]
    trace: Option<PathBuf>,
    /// Blacklist: listed toolchains will not be processed
    #[structopt(long)]
    skip: Vec<String>,
    /// Whitelist: only listed toolchains will be processed (overrides `skip`)
    #[structopt(long)]
    only: Vec<String>,
    /// Do not treat symlinks like regular files
    #[structopt(long)]
    copy_symlinks: bool,

    /// Allow copying directories
    #[structopt(long)]
    copy_dirs: bool,

    /// Print files instead of copying
    ///
    /// Files will be printed to provided file instead of copying
    #[structopt(long)]
    print: Option<PathBuf>,
}

fn run_under_trace(
    script_path: &Path,
    data_path: &Path,
    detect_out: &DetectScriptOutput,
    work_dir: &Path,
) -> anyhow::Result<Vec<u8>> {
    println!("running in {}", work_dir.display());
    let log_out_file = work_dir.join("__jjs_trace.json");
    let data_path = data_path.canonicalize().context("data dir not exists")?;
    println!("script will use data from {}", data_path.display());
    let mut cmd = Command::new("lxtrace");
    cmd.current_dir(work_dir)
        // machine-readable
        .arg("--json")
        // redirect to file, so it will not mix with script output
        .arg("--inherit-env")
        .arg("--file")
        .arg(&log_out_file)
        .arg("--")
        .arg("bash")
        .arg(script_path.canonicalize().context("script not exists")?)
        .env("DATA", data_path);
    for (k, v) in &detect_out.env {
        cmd.env(k, v);
    }
    let status = cmd.status().context("failed to start ktrace")?;
    if !status.success() {
        anyhow::bail!("ktrace returned error");
    }
    Ok(std::fs::read(&log_out_file).context("failed to read trace log")?)
}

#[derive(Clone)]
struct TemplateInfo {
    dir: PathBuf,
    name: String,
    cfg: config::ToolchainConfig,
}
mod tpl_info_impls {
    use super::*;
    use std::{cmp::*, hash::*};

    impl Hash for TemplateInfo {
        fn hash<H: Hasher>(&self, hasher: &mut H) {
            self.name.hash(hasher);
        }
    }

    impl PartialEq for TemplateInfo {
        fn eq(&self, that: &TemplateInfo) -> bool {
            self.name == that.name
        }
    }

    impl Eq for TemplateInfo {}
}

fn list_templates(dir: &Path) -> anyhow::Result<Vec<TemplateInfo>> {
    let content = std::fs::read_dir(dir).context("failed to read toolchain templates dir")?;
    let mut out = Vec::new();
    for item in content {
        let item = item.context("failed to stat toolchain template dir")?;
        let cfg = item.path().join("config.toml");
        let cfg = std::fs::read_to_string(cfg).context("failed to open manifest")?;
        let cfg = toml::from_str(&cfg).context("failed to parse manifest")?;
        let name = item
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .context("toolchain name is not utf8")?
            .to_string();
        out.push(TemplateInfo {
            dir: item.path(),
            name,
            cfg,
        });
    }
    Ok(out)
}

fn select_templates(
    tpls: impl Iterator<Item = TemplateInfo>,
    opt: &Options,
) -> anyhow::Result<impl Iterator<Item = TemplateInfo>> {
    let filter: Box<dyn FnMut(&TemplateInfo) -> bool> = if !opt.only.is_empty() {
        Box::new(|tpl| opt.only.contains(&tpl.name) || tpl.cfg.auto)
    } else if !opt.skip.is_empty() {
        Box::new(|tpl| !opt.skip.contains(&tpl.name))
    } else {
        Box::new(|_tpl| true)
    };
    let tpls: Vec<_> = tpls.collect();
    let roots: Vec<_> = tpls.clone().into_iter().filter(filter).collect();
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
    Ok(used.into_iter())
}

struct DetectScriptOutput {
    env: std::collections::HashMap<String, String>,
}

impl std::str::FromStr for DetectScriptOutput {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut this = Self {
            env: std::collections::HashMap::new(),
        };
        for line in s.lines() {
            if line.starts_with("set-env:") {
                let cmd = line.trim_start_matches("set-env:");
                let parts: Vec<_> = cmd.splitn(2, '=').collect();
                if parts.len() != 2 {
                    bail!("set-env command does not look like var_name=var_value");
                }
                this.env.insert(parts[0].to_string(), parts[1].to_string());
            } else {
                bail!("unknown command: {}", line);
            }
        }
        Ok(this)
    }
}

fn run_detect_script(
    tpl: &TemplateInfo,
    data_dir: &Path,
    work_dir: &Path,
) -> anyhow::Result<Option<DetectScriptOutput>> {
    let detect_script_path = tpl.dir.join("detect.sh");
    if !detect_script_path.exists() {
        bail!("detect.sh script missing");
    }
    let out_file_path = tempfile::NamedTempFile::new().context("failed to allocate temp file")?;

    let status = std::process::Command::new("bash")
        .arg(&detect_script_path)
        .arg(out_file_path.path())
        .current_dir(work_dir)
        .env("DATA", data_dir)
        .status()
        .context("failed to execute detect.sh script")?;
    let script_out =
        std::fs::read_to_string(out_file_path.path()).context("failed to read detect.sh output")?;
    println!("--- script control output ---");
    print!("{}", &script_out);
    println!("--- end script control output ---");
    let script_out = script_out
        .parse()
        .context("failed to parse detect.sh output")?;
    let script_out = if status.success() {
        Some(script_out)
    } else {
        None
    };
    Ok(script_out)
}

fn process_toolchain_invoke_conf(
    tpl: &TemplateInfo,
    out_dir: &Path,
    detect_out: &DetectScriptOutput,
) -> anyhow::Result<()> {
    let out_file = out_dir
        .join("etc/objects/toolchains")
        .join(format!("{}.yaml", &tpl.name));
    let in_file = tpl.dir.join("invoke-conf.yaml");
    if !in_file.exists() {
        return Ok(());
    }
    let in_file = std::fs::read_to_string(&in_file).context("failed to read invoke-conf.yaml")?;

    let mut render_ctx = tera::Context::new();
    for (k, v) in &detect_out.env {
        let k = format!("env_{}", k);
        render_ctx.insert(k, v);
    }
    let output = tera::Tera::one_off(&in_file, &render_ctx, false)
        .context("failed to render invoke config file")?;

    std::fs::create_dir_all(out_file.parent().unwrap()).ok();

    std::fs::write(&out_file, &output).context("failed to create config file")?;

    Ok(())
}

fn process_toolchain_template(
    tpl: TemplateInfo,
    collector: &mut DepCollector,
    mut event_log: Option<&mut dyn std::io::Write>,
    out_dir: &Path,
) -> anyhow::Result<()> {
    let work_dir = tempfile::TempDir::new().context("failed to create temp dir")?;
    let data_dir = tpl.dir.join("data");
    let detect_out = match run_detect_script(&tpl, &data_dir, work_dir.path())? {
        Some(dso) => dso,
        None => {
            println!("Skipping toolchain {}: not available", &tpl.name);
            return Ok(());
        }
    };
    let scripts: anyhow::Result<Vec<_>> = tpl
        .dir
        .join("use")
        .read_dir()?
        .map(|item| item.map_err(|err| anyhow::Error::new(err).context("failed to read script")))
        .collect();
    for script in scripts? {
        println!("running {}", script.path().display());
        let out = run_under_trace(&script.path(), &data_dir, &detect_out, work_dir.path())
            .context("failed to collect trace")?;
        let scanner = serde_json::Deserializer::from_slice(&out).into_iter();
        let mut cnt = 0;
        let mut cnt_items = 0;
        let mut cnt_errors = 0;
        for val in scanner {
            let val: serde_json::Value = val.context("failed to parse ktrace output")?;
            if let Some(mut wr) = event_log.as_mut() {
                serde_json::to_writer(&mut wr, &val).ok();
                writeln!(&mut wr).ok();
            }
            if val
                .pointer("/payload/data/decoded")
                .map(|val| val.is_null())
                .unwrap_or(false)
            {
                continue;
            }
            cnt += match collector.process_log_item(&val).with_context(|| {
                format!(
                    "failed to process output item: {}",
                    serde_json::to_string_pretty(&val).unwrap()
                )
            }) {
                Ok(cnt) => cnt,
                Err(err) => {
                    util::print_error(&*err);
                    cnt_errors += 1;
                    0
                }
            };
            cnt_items += 1;
        }
        println!(
            "script processed: {} trace events, {} new files, {} errors",
            cnt_items, cnt, cnt_errors
        );
    }
    process_toolchain_invoke_conf(&tpl, &out_dir, &detect_out)?;
    Ok(())
}

fn main_inner() -> anyhow::Result<()> {
    let opt: Options = Options::from_args();
    let mut collector = dep_collector::DepCollector::default();
    let mut log_file = match &opt.trace {
        Some(path) => {
            let wr = std::fs::File::create(path).context("failed to open trace log")?;
            let wr = std::io::BufWriter::new(wr);
            Some(wr)
        }
        None => None,
    };
    let templates = list_templates(&opt.tpls_dir)?;
    let templates = select_templates(templates.into_iter(), &opt)?;
    for tpl in templates {
        println!("------ processing {} ------", &tpl.name);

        if let Err(e) = process_toolchain_template(
            tpl,
            &mut collector,
            log_file.as_mut().map(|x| x as _),
            &opt.out,
        ) {
            util::print_error(&*e);
        }
    }
    println!(
        "all toolchains processed: {} files found",
        collector.count()
    );
    let toolchain_files_output_dir = opt.out.join("opt");
    let mut process_file: Box<dyn FnMut(&str)> = if let Some(path) = &opt.print {
        let out_file = std::fs::File::create(&path).context("failed to open output file")?;
        let mut out_file = std::io::BufWriter::new(out_file);
        Box::new(move |file| {
            use std::io::Write;
            writeln!(&mut out_file, "{}", file).ok();
        })
    } else {
        Box::new(move |file| {
            let file = Path::new(&file);
            if file.is_dir() && !opt.copy_dirs {
                return;
            }
            if let Err(e) =
                copy_ln::copy(&toolchain_files_output_dir, file, true, !opt.copy_symlinks)
            {
                eprintln!("{:?}", e);
            }
        })
    };
    for file in collector {
        if std::fs::canonicalize(&file).is_err() {
            // ignore file if it does not exist.
            continue;
        }
        if file.starts_with("/tmp") || file.starts_with("/dev") || file.starts_with("/home") {
            continue;
        }
        process_file(&file);
    }
    Ok(())
}

fn main() -> ExitCode {
    if let Err(e) = main_inner() {
        eprintln!("error: {}", e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
