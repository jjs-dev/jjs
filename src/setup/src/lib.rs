use anyhow::Context;
use log::{error, info};
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
    pub drop_existing: bool,
}

pub struct SetupParams {
    pub data_dir: Option<PathBuf>,
    pub install_dir: PathBuf,
    pub db: Option<DatabaseParams>,
    pub config: Option<ConfigParams>,
    pub sample_contest: bool,
    pub force: bool,
    pub toolchains: bool,
}

fn add(data_dir: &Path, name: &str) -> anyhow::Result<()> {
    let p = data_dir.join(name);
    info!("creating {}", p.display());
    fs::create_dir(p)?;
    Ok(())
}

fn create_dirs(params: &SetupParams) -> anyhow::Result<()> {
    if let Some(data_dir) = &params.data_dir {
        if params.force && data_dir.exists() {
            std::fs::remove_dir_all(&data_dir).context("failed to remove existing data_dir")?;
        }
        std::fs::create_dir(&data_dir).ok();
        {
            let mut dir = fs::read_dir(&data_dir)?.peekable();
            if dir.peek().is_some() {
                error!("Specified dir is not empty");
                for item in dir.by_ref().take(10) {
                    let item_path = item
                        .map(|it| it.path().display().to_string())
                        .unwrap_or_else(|err| format!("<{}>", err));
                    error!("- {}", item_path);
                }
                let rem = dir.count();
                if rem > 0 {
                    error!("- And {} more", rem);
                }
                process::exit(1);
            }
        }

        add(data_dir, "var")?;
        add(data_dir, "var/runs")?;
        add(data_dir, "var/problems")?;
        add(data_dir, "opt")?;
    }
    Ok(())
}

fn copy_or_symlink_config(conf_params: &ConfigParams, params: &SetupParams) -> anyhow::Result<()> {
    let data_dir = match params.data_dir.as_ref() {
        Some(d) => d,
        None => return Ok(()),
    };
    info!("writing config to {}", data_dir.join("etc").display());
    let cfg_dir = params.install_dir.join("example-config");
    if conf_params.symlink {
        let symlink_target = data_dir.join("etc");
        let symlink_src = fs::canonicalize(&cfg_dir)?;
        std::os::unix::fs::symlink(symlink_src, symlink_target)?;
    } else {
        add(data_dir, "etc")?;
        add(data_dir, "etc/objects")?;
        let cfg_dir_items = vec!["objects", "apiserver.yaml"]
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
) -> anyhow::Result<()> {
    let conn_url = url::Url::parse(&db_params.uri).expect("db connection string is ill-formed");
    let migrate_script_path = params.install_dir.join("share/db-setup.sql");
    let db_name = conn_url.path().trim_start_matches('/');
    let host = conn_url.host().expect("db hostname missing");
    let port = conn_url.port().unwrap_or(5432);
    let host_arg = if host.to_string() == "localhost" {
        None
    } else {
        Some(format!("--host={}", &host))
    };
    if db_params.drop_existing {
        info!("Dropping DB {}", &db_name);
        let mut cmd = Command::new("dropdb");
        cmd.arg(db_name);
        if let Some(host_arg) = &host_arg {
            cmd.arg(host_arg);
        }
        cmd.arg(format!("--port={}", port))
            .arg("--no-password")
            .try_exec()
            .ok();
    }

    info!("Creating DB {}", &db_name);
    {
        let mut cmd = Command::new("createdb");
        cmd.arg(db_name);
        if let Some(host_arg) = &host_arg {
            cmd.arg(host_arg);
        }
        cmd.arg(format!("--port={}", port))
            .arg("--no-password")
            .try_exec()?;
    }
    let psql = || {
        let mut cmd = Command::new("psql");
        cmd.arg(format!("--dbname={}", &db_params.uri));
        cmd
    };
    info!("Running migrations");
    {
        let mut cmd = psql();
        cmd.arg(format!("--file={}", migrate_script_path.display()));
        cmd.arg("--no-password");
        cmd.run_on(runner);
    }
    info!("Creating user");
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

fn setup_contest(params: &SetupParams) -> anyhow::Result<()> {
    if let Some(data_dir) = &params.data_dir {
        let out_path = data_dir.join("var/problems");
        let src_dir = params.install_dir.join("example-problems");
        let build_problem = |prob_name: &str| -> anyhow::Result<()> {
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
                error!("failed to build problem {}", prob_name);
            }
            Ok(())
        };
        build_problem("a-plus-b")?;
        build_problem("array-sum")?;
        build_problem("sqrt")?;
    }
    Ok(())
}

fn setup_toolchains(params: &SetupParams) -> anyhow::Result<()> {
    let conf_tcs_path = params.install_dir.join("bin/jjs-configure-toolchains");
    let mut cmd = Command::new(conf_tcs_path);
    let toolchain_spec_db_dir = params.install_dir.join("toolchains");
    let target_dir = match params.data_dir.as_ref() {
        Some(d) => d,
        None => return Ok(()),
    };
    cmd.arg(toolchain_spec_db_dir);
    cmd.arg(target_dir);
    cmd.arg("--trace")
        .arg(target_dir.join("configure-toolchains-log.txt"));
    let st = cmd.status()?.success();
    if !st {
        anyhow::bail!("failed to run jjs-configure-toolchains")
    }
    Ok(())
}

pub fn setup(params: &SetupParams, runner: &Runner) -> anyhow::Result<()> {
    std::env::set_var("JJS_PATH", &params.install_dir);
    if let Some(data_dir) = &params.data_dir {
        std::env::set_var("JJS_SYSROOT", data_dir);
    }
    create_dirs(params)?;
    if let Some(conf_params) = &params.config {
        info!("setting up config");
        copy_or_symlink_config(conf_params, params)?;
    }
    if let Some(db_params) = &params.db {
        info!("setting up DB");
        setup_db(db_params, params, runner)?;
    }
    if params.sample_contest {
        info!("setting up sample contests");
        setup_contest(params)?;
    }
    if params.toolchains {
        info!("configuring toolchains");
        setup_toolchains(params)?;
    }
    Ok(())
}
