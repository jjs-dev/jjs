use std::{fs, path::PathBuf, process, process::Command};

pub struct ConfigParams {
    pub symlink: bool,
}

pub struct DatabaseParams {
    pub uri: String,
}

pub struct SetupParams {
    pub data_dir: PathBuf,
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

fn add(params: &SetupParams, name: &str) -> Result<(), SetupError> {
    let p = params.data_dir.join(name);
    fs::create_dir(p)?;
    Ok(())
}

fn create_dirs(params: &SetupParams) -> Result<(), SetupError> {
    if params.force {
        std::fs::remove_dir_all(&params.data_dir).ok();
    }
    std::fs::create_dir(&params.data_dir).ok();
    {
        let mut dir = fs::read_dir(&params.data_dir)?;
        if dir.next().is_some() {
            eprintln!("Specified dir is not empty");
            process::exit(1);
        }
    }

    add(params, "var")?;
    add(params, "var/submissions")?;
    add(params, "var/problems")?;
    add(params, "opt")?;
    add(params, "opt/bin")?;
    add(params, "opt/lib64")?;
    add(params, "opt/lib")?;
    add(params, "opt/etc")?;
    Ok(())
}

fn copy_or_symlink_config(
    conf_params: &ConfigParams,
    params: &SetupParams,
) -> Result<(), SetupError> {
    let cfg_dir = params.install_dir.join("example-config");
    if conf_params.symlink {
        let symlink_target = params.data_dir.join("etc");
        let symlink_src = fs::canonicalize(&cfg_dir)?;
        std::os::unix::fs::symlink(symlink_src, symlink_target)?;
    } else {
        add(params, "etc")?;
        add(params, "etc/toolchains")?;
        let cfg_dir_items = vec!["jjs.toml", "toolchains", "contest.toml"]
            .iter()
            .map(|x| cfg_dir.join(x))
            .collect();
        fs_extra::copy_items(
            &cfg_dir_items,
            params.data_dir.join("etc"),
            &fs_extra::dir::CopyOptions::new(),
        )?;
    }
    Ok(())
}

fn setup_db(db_params: &DatabaseParams, params: &SetupParams) -> Result<(), SetupError> {
    let migrate_script_path = params.install_dir.join("share/db-setup.sql");
    {
        Command::new("createdb")
            .arg("jjs") // TODO: take from params
            .status()?;
    }
    {
        let mut cmd = Command::new("psql");
        cmd.arg(format!("--file={}", migrate_script_path.display()));
        cmd.arg(format!("--dbname={}", &db_params.uri));
        let st = cmd.status()?;
        if !st.success() {
            panic!("psql failed");
        }
    }
    Ok(())
}

fn setup_contest(params: &SetupParams) -> Result<(), SetupError> {
    let out_path = params.data_dir.join("var/problems");
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
    Ok(())
}

pub fn setup(params: &SetupParams) -> Result<(), SetupError> {
    std::env::set_var("JJS_PATH", &params.install_dir);
    std::env::set_var("JJS_SYSROOT", &params.data_dir);
    create_dirs(params)?;
    if let Some(conf_params) = &params.config {
        copy_or_symlink_config(conf_params, params)?;
    }
    if let Some(db_params) = &params.db {
        setup_db(db_params, params)?;
    }
    if params.sample_contest {
        setup_contest(params)?;
    }
    Ok(())
}
