use std::fs;

pub fn make_empty(path: &str) -> Result<(), std::io::Error> {
    let path = std::path::PathBuf::from(path);
    if path.exists() {
        for item in fs::read_dir(&path)? {
            let path = item?.path();
            if path.is_dir() {
                fs::remove_dir_all(path)?
            } else {
                fs::remove_file(path)?
            }
        }
    } else {
        fs::create_dir_all(path)?;
    }

    Ok(())
}

#[derive(Debug)]
struct MessageError(String);

impl std::fmt::Display for MessageError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str(&self.0)
    }
}

impl From<MessageError> for std::io::Error {
    fn from(e: MessageError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    }
}

impl std::error::Error for MessageError {}

pub fn create_or_empty(path: &str) -> Result<(), std::io::Error> {
    let path = std::path::PathBuf::from(path);
    if path.exists() {
        if !path.is_dir() {
            Err(MessageError("path is not directory".to_string()))?;
        }
        if path.read_dir()?.next().is_some() {
            Err(MessageError("path is not empty".to_string()))?
        }
        Ok(())
    } else {
        std::fs::create_dir_all(path)
    }
}

pub fn get_primary_style() -> console::Style {
    console::Style::new().green()
}

pub fn print_section(section: &str) {
    let msg = format!("----> {}", section);
    println!("{}", get_primary_style().apply_to(msg));
}

pub fn ensure_exists(path: &str) -> Result<(), std::io::Error> {
    use std::io::ErrorKind::*;
    match fs::create_dir_all(path) {
        Ok(_) => (),
        Err(e) => match e.kind() {
            AlreadyExists => (),
            _ => return Err(e),
        },
    };

    Ok(())
}

pub fn get_current_target() -> String {
    //provided by build.rs
    env!("TARGET").to_owned()
}
