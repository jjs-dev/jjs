use anyhow::Context;
use std::path::PathBuf;

pub struct CfgData {
    pub data_dir: PathBuf,
    pub install_dir: PathBuf,
    pub entity_loader: entity::Loader,
    pub problem_loader: problem_loader::Loader,
}

fn find_data_dir() -> anyhow::Result<PathBuf> {
    match std::env::var_os("JJS_DATA") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Err(anyhow::anyhow!("JJS_DATA env var is missing")),
    }
}

fn find_install_dir() -> anyhow::Result<PathBuf> {
    match std::env::var_os("JJS_PATH") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Err(anyhow::anyhow!("JJS_PATH env var is missing")),
    }
}

pub fn load_cfg_data() -> anyhow::Result<CfgData> {
    let data_dir = find_data_dir().context("failed to find data dir")?;
    let install_dir = find_install_dir().context("failed to find installation dir")?;
    let entity_loader =
        entity::Loader::load_from_data_dir(&data_dir).context("failed to load entities")?;
    let problem_loader =
        problem_loader::Loader::load_from_data_dir(&data_dir).context("failed to load problems")?;
    Ok(CfgData {
        entity_loader,
        problem_loader,
        data_dir,
        install_dir,
    })
}
