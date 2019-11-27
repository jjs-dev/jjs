#![feature(process_exitcode_placeholder)]

mod config;
mod dep_collector;

use crate::dep_collector::DepCollector;
use anyhow::Context;
use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    /// Spec files dir
    spec: PathBuf,
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

fn run_under_trace(script_path: &Path, data_path: &Path) -> anyhow::Result<Vec<u8>> {
    let current_dir = tempfile::TempDir::new().context("failed to create temp dir")?;
    println!("running in {}", current_dir.path().display());
    let log_out_file = current_dir.path().join("__jjs_trace.json");
    let data_path = data_path.canonicalize().context("data dir not exists")?;
    println!("script will use data from {}", data_path.display());
    let status = Command::new("lxtrace")
        .current_dir(current_dir.path())
        // machine-readable
        .arg("--json")
        // redirect to file, so it will not mix with script output
        .arg("--inherit-env")
        .arg("--file")
        .arg(&log_out_file)
        .arg("--")
        .arg("bash")
        .arg(script_path.canonicalize().context("script not exists")?)
        .env("DATA", data_path)
        .status()
        .context("failed to start ktrace")?;
    if !status.success() {
        anyhow::bail!("ktrace returned error");
    }
    Ok(std::fs::read(&log_out_file).context("failed to read trace log")?)
}

fn process_toolchain(
    dir: &Path,
    collector: &mut DepCollector,
    mut event_log: Option<&mut dyn std::io::Write>,
) -> anyhow::Result<()> {
    let manifest_path = dir.join("config.toml");
    let manifest =
        std::fs::read_to_string(manifest_path).context("config.toml not found or not readable")?;
    let _manifest: config::Config = toml::from_str(&manifest).context("failed to parse config")?;
    // TODO: look at config
    let scripts: anyhow::Result<Vec<_>> = dir
        .join("scripts")
        .read_dir()?
        .map(|item| item.map_err(|err| anyhow::Error::new(err).context("failed to read script")))
        .collect();
    let current_dir = dir.join("data");
    for script in scripts? {
        println!("running {}", script.path().display());
        let out =
            run_under_trace(&script.path(), &current_dir).context("failed to collect trace")?;
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
    for item in std::fs::read_dir(&opt.spec).context("failed read spec dir")? {
        let item = item.context("failed read spec dir")?;
        let title = item
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .context("toolchain name is not utf8")?
            .to_string();

        let mut ok = true;
        if !opt.only.is_empty() {
            ok = opt.only.contains(&title);
        } else if opt.skip.contains(&title) {
            ok = false;
        }
        if ok {
            println!("processing {}", &title);
        } else {
            println!("skipping {}", &title);
            continue;
        }
        if let Err(e) = process_toolchain(
            &item.path(),
            &mut collector,
            log_file.as_mut().map(|x| x as _),
        ) {
            util::print_error(&*e);
        }
    }
    println!(
        "all toolchains processed: {} files found",
        collector.count()
    );
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
            if let Err(e) = copy_ln::copy(&opt.out, file, true, !opt.copy_symlinks) {
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
