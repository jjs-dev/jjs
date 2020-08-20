use anyhow::Context;
use std::path::PathBuf;

#[derive(Debug)]
pub struct CfgData {
    pub data_dir: PathBuf,
}

fn find_data_dir() -> anyhow::Result<PathBuf> {
    match std::env::var_os("JJS_DATA") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Err(anyhow::anyhow!("JJS_DATA env var is missing")),
    }
}

pub fn load_cfg_data() -> anyhow::Result<CfgData> {
    let data_dir = find_data_dir().context("failed to find data dir")?;
    Ok(CfgData { data_dir })
}
