use std::{
    fs,
    path::{Path, PathBuf},
    process,
    process::Command,
};
use util::cmd::{CommandExt, Runner};

pub struct ConfigParams {
    pub symlink: bool,
}

pub struct DatabaseParams {
    pub uri: String,
}

pub struct SetupParams {
    pub data_dir: Option<PathBuf>,
    pub install_dir: PathBuf,
    pub db: Option<DatabaseParams>,
    pub config: Option<ConfigParams>,
    pub sample_contest: bool,
    pub force: bool,
}

#[derive(Debug)]
pub struct SetupError {
    pub source: Box<dyn std::error::Error>,
    pub backtrace: backtrace::Backtrace,
}

impl<E: std::error::Error + 'static> From<E> for SetupError {
    fn from(source: E) -> Self {
        Self {
            source: Box::new(source),
            backtrace: backtrace::Backtrace::new(),
        }
    }
}

fn add(data_dir: &Path, name: &str) -> Result<(), SetupError> {
    let p = data_dir.join(name);
    fs::create_dir(p)?;
    Ok(())
}

fn create_dirs(params: &SetupParams) -> Result<(), SetupError> {
    if let Some(data_dir) = &params.data_dir {
        if params.force {
            std::fs::remove_dir_all(&data_dir).ok();
        }
        std::fs::create_dir(&data_dir).ok();
        {
            let mut dir = fs::read_dir(&data_dir)?;
            if dir.next().is_some() {
                eprintln!("Specified dir is not empty");
                process::exit(1);
            }
        }

        add(data_dir, "var")?;
        add(data_dir, "var/submissions")?;
        add(data_dir, "var/problems")?;
        add(data_dir, "opt")?;
        add(data_dir, "opt/bin")?;
        add(data_dir, "opt/lib64")?;
        add(data_dir, "opt/lib")?;
        add(data_dir, "opt/etc")?;
    }
    Ok(())
}

fn copy_or_symlink_config(
    conf_params: &ConfigParams,
    params: &SetupParams,
) -> Result<(), SetupError> {
    let data_dir = match params.data_dir.as_ref() {
        Some(d) => d,
        None => return Ok(()),
    };
    let cfg_dir = params.install_dir.join("example-config");
    if conf_params.symlink {
        let symlink_target = data_dir.join("etc");
        let symlink_src = fs::canonicalize(&cfg_dir)?;
        std::os::unix::fs::symlink(symlink_src, symlink_target)?;
    } else {
        add(data_dir, "etc")?;
        add(data_dir, "etc/toolchains")?;
        let cfg_dir_items = vec!["jjs.toml", "toolchains", "contest.toml"]
            .iter()
            .map(|x| cfg_dir.join(x))
            .collect();
        fs_extra::copy_items(
            &cfg_dir_items,
            data_dir.join("etc"),
            &fs_extra::dir::CopyOptions::new(),
        )?;
    }
    Ok(())
}

fn setup_db(
    db_params: &DatabaseParams,
    params: &SetupParams,
    runner: &Runner,
) -> Result<(), SetupError> {
    let conn_url = url::Url::parse(&db_params.uri).expect("db connection string is ill-formed");
    let migrate_script_path = params.install_dir.join("share/db-setup.sql");
    log::info!("Creating DB");
    let host = conn_url.host().expect("db hostname missing");
    let port = conn_url.port().unwrap_or(5432);
    {
        Command::new("createdb")
            .arg(conn_url.path().trim_start_matches('/')) // TODO: take from params
            .arg(format!("--host={}", &host))
            .arg(format!("--port={}", &port))
            .status()?;
    }
    let psql = || {
        let mut cmd = Command::new("psql");
        cmd.arg(format!("--dbname={}", &db_params.uri));
        cmd
    };
    log::info!("Running migrations");
    {
        let mut cmd = psql();
        cmd.arg(format!("--file={}", migrate_script_path.display()));
        cmd.run_on(runner);
    }
    log::info!("Creating user");
    {
        let mut cmd = Command::new("createuser");
        cmd.arg("--superuser");
        cmd.arg(format!("--host={}", &host));
        cmd.arg(format!("--port={}", &port));
        cmd.arg("--no-password");
        cmd.arg("root");
    }
    Ok(())
}

fn setup_contest(params: &SetupParams) -> Result<(), SetupError> {
    if let Some(data_dir) = &params.data_dir {
        let out_path = data_dir.join("var/problems");
        let src_dir = params.install_dir.join("example-problems");
        let build_problem = |prob_name: &str| -> Result<(), SetupError> {
            let out_path = out_path.join(prob_name);
            std::fs::create_dir(&out_path)?;
            let src_dir = src_dir.join(prob_name);
            let ppc_path = params.install_dir.join("bin/jjs-ppc");
            let mut cmd = Command::new(ppc_path);
            cmd.arg("compile");
            cmd.arg("--out").arg(out_path);
            cmd.arg("--pkg").arg(src_dir);
            let st = cmd.status()?.success();
            if !st {
                eprintln!("Error: failed build problem {}", prob_name);
            }
            Ok(())
        };
        build_problem("a-plus-b")?;
        build_problem("array-sum")?;
        build_problem("sqrt")?;
    }
    Ok(())
}

pub fn setup(params: &SetupParams, runner: &Runner) -> Result<(), SetupError> {
    std::env::set_var("JJS_PATH", &params.install_dir);
    if let Some(data_dir) = &params.data_dir {
        std::env::set_var("JJS_SYSROOT", data_dir);
    }
    create_dirs(params)?;
    if let Some(conf_params) = &params.config {
        log::info!("setting up config");
        copy_or_symlink_config(conf_params, params)?;
    }
    if let Some(db_params) = &params.db {
        log::info!("setting up DB");
        setup_db(db_params, params, runner)?;
    }
    if params.sample_contest {
        log::info!("setting up sample contests");
        setup_contest(params)?;
    }
    Ok(())
}
