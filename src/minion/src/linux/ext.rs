use nix::mount::MsFlags;
use std::path::Path;
use thiserror::Error;

pub struct Quota(u64);

const MEBIBYTE: u64 = 1u64 << 20;

impl Quota {
    pub fn bytes(bytes: u64) -> Quota {
        Quota(bytes)
    }

    pub fn mebibytes(mibs: u64) -> Quota {
        Quota(mibs * MEBIBYTE)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("syscall error: {0}")]
    Syscall(#[from] nix::Error),
    #[error("failed to create dir: {0}")]
    CreateDir(std::io::Error),
}

pub fn make_tmpfs(path: &Path, quota: Quota) -> Result<(), Error> {
    let options = format!("size={}", quota.0);
    if let Err(err) = std::fs::create_dir(path) {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(Error::CreateDir(err));
        }
    }
    // return Ok(());
    nix::mount::mount(
        None::<&str>,
        path,
        Some("tmpfs"),
        MsFlags::empty(),
        Some(options.as_bytes()),
    )?;
    Ok(())
}
