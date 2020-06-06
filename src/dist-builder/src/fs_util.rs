use std::{fs, path::Path};

pub fn ensure_exists(path: impl AsRef<Path>) -> anyhow::Result<()> {
    use std::io::ErrorKind::*;
    match fs::create_dir_all(path) {
        Ok(_) => (),
        Err(e) => match e.kind() {
            AlreadyExists => (),
            _ => return Err(e.into()),
        },
    };

    Ok(())
}
