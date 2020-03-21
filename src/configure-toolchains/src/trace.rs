mod dep_collector;

use super::ToolchainSpec;
use crate::config::Strategy;
use anyhow::{bail, Context as _};
use dep_collector::DepCollector;
use std::{
    io::{Read as _, Write as _},
    path::Path,
    process::Command,
};

#[derive(Clone, Copy)]
enum Tracer {
    Strace,
    Lxtrace,
}

fn run_under_trace(
    script_path: &Path,
    data_path: &Path,
    detect_out: &DetectScriptOutput,
    work_dir: &Path,
    tracer: Tracer,
) -> anyhow::Result<Vec<u8>> {
    println!("Running in {}", work_dir.display());
    let log_out_file = work_dir.join("__jjs_trace.json");
    let data_path = data_path.canonicalize().context("data dir not exists")?;
    println!("Script will use data from {}", data_path.display());
    let script_path = script_path.canonicalize().context("script not exists")?;
    match tracer {
        Tracer::Lxtrace => {
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
                .arg(&script_path)
                .env("DATA", data_path);
            for (k, v) in &detect_out.env {
                cmd.env(k, v);
            }
            let status = cmd.status().context("failed to start lxtrace")?;
            if !status.success() {
                anyhow::bail!("lxtrace returned error");
            }
            Ok(std::fs::read(&log_out_file).context("failed to read trace log")?)
        }
        Tracer::Strace => {
            let mut strace = Command::new("strace");
            strace
                .current_dir(&work_dir)
                .arg("-f")
                .arg("-o")
                .arg(&log_out_file)
                .arg("-s")
                .arg("300")
                .arg("bash")
                .arg(&script_path)
                .env("DATA", data_path);
            for (k, v) in &detect_out.env {
                strace.env(k, v);
            }
            let status = strace.status().context("failed to start strace")?;
            if !status.success() {
                anyhow::bail!("strace failed");
            }

            let parse_strace_path =
                std::path::PathBuf::from(std::env::var("JJS_PATH").context("JJS_PATH not exists")?)
                    .join("libexec/strace-parser.py");

            let mut parser = Command::new("python3");
            parser.arg(&parse_strace_path);
            parser.arg("--lxtrace");
            parser.arg(format!("--input={}", log_out_file.display()));
            parser.stdin(std::process::Stdio::null());
            parser.stdout(std::process::Stdio::piped());
            parser.stderr(std::process::Stdio::inherit());
            let mut parser = parser.spawn().context("failed to spawn parser")?;
            let mut parsed = Vec::new();
            parser
                .stdout
                .take()
                .unwrap()
                .read_to_end(&mut parsed)
                .context("read parsed trace")?;
            parser.kill().ok();
            Ok(parsed)
        }
    }
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
    tpl: &ToolchainSpec,
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
    tpl: &ToolchainSpec,
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

fn process_toolchain_spec(
    tpl: &ToolchainSpec,
    collector: &mut DepCollector,
    mut event_log: Option<&mut dyn std::io::Write>,
    out_dir: &Path,
    strace: bool,
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
    let tracer = if strace {
        Tracer::Strace
    } else {
        Tracer::Lxtrace
    };
    let scripts: anyhow::Result<Vec<_>> = tpl
        .dir
        .join("use")
        .read_dir()?
        .map(|item| item.map_err(|err| anyhow::Error::new(err).context("failed to read script")))
        .collect();
    for script in scripts? {
        println!("Running {}", script.path().display());
        let out = run_under_trace(
            &script.path(),
            &data_dir,
            &detect_out,
            work_dir.path(),
            tracer,
        )
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
            "Script processed: {} trace events, {} new files, {} errors",
            cnt_items, cnt, cnt_errors
        );
    }
    process_toolchain_invoke_conf(&tpl, &out_dir, &detect_out)?;
    Ok(())
}

pub(crate) struct TraceResolver {
    collector: DepCollector,
    options: crate::Options,
}

impl TraceResolver {
    pub(crate) fn new(opts: &crate::Options) -> TraceResolver {
        TraceResolver {
            collector: DepCollector::default(),
            options: opts.clone(),
        }
    }
}

impl crate::Resolver for TraceResolver {
    fn strategy_name(&self) -> &'static str {
        "trace"
    }

    fn strategy(&self) -> Strategy {
        Strategy::Trace
    }

    fn visit_spec(
        &mut self,
        spec: &ToolchainSpec,
        log_file: Option<&mut dyn std::io::Write>,
    ) -> anyhow::Result<()> {
        process_toolchain_spec(
            spec,
            &mut self.collector,
            log_file,
            &self.options.out,
            !self.options.lxtrace,
        )
    }

    fn finish(&mut self) -> anyhow::Result<()> {
        let collector = std::mem::take(&mut self.collector);
        let toolchain_files_output_dir = self.options.out.join("opt");
        let mut process_file: Box<dyn FnMut(&str)> = if let Some(path) = &self.options.dry_run {
            let out_file = std::fs::File::create(&path).context("failed to open output file")?;
            let mut out_file = std::io::BufWriter::new(out_file);
            Box::new(move |file| {
                use std::io::Write;
                writeln!(&mut out_file, "{}", file).ok();
            })
        } else {
            Box::new(move |file| {
                let file = Path::new(&file);
                if file.is_dir() && !self.options.copy_dirs {
                    return;
                }
                if let Err(e) = copy_ln::copy(
                    &toolchain_files_output_dir,
                    file,
                    true,
                    !self.options.copy_symlinks,
                ) {
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
}
