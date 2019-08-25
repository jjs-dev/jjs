use std::{fs, process};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Args {
    pub sysroot_dir: String,
    pub config_dir: Option<String>,
    #[structopt(long = "symlink-config")]
    pub symlink_config: bool,
}

#[derive(Debug)]
pub struct InitError {
    pub source: Box<dyn std::error::Error>,
    pub backtrace: backtrace::Backtrace,
}

impl<E: std::error::Error + 'static> From<E> for InitError {
    fn from(source: E) -> Self {
        Self {
            source: Box::new(source),
            backtrace: backtrace::Backtrace::new(),
        }
    }
}

pub fn init_jjs_root(args: Args) -> Result<(), InitError> {
    let path = &args.sysroot_dir;
    {
        let mut dir = fs::read_dir(path)?;
        if dir.next().is_some() {
            eprintln!("Specified dir is not empty");
            process::exit(1);
        }
    }

    let add = |name: &str| -> Result<(), InitError> {
        let p = format!("{}/{}", path, name);
        fs::create_dir(p)?;
        Ok(())
    };

    add("var")?;
    add("var/submissions")?;
    add("var/problems")?;
    add("opt")?;
    add("opt/bin")?;
    add("opt/lib64")?;
    add("opt/lib")?;
    add("opt/etc")?;
    if let Some(cfg_dir) = &args.config_dir {
        if args.symlink_config {
            let symlink_target = format!("{}/etc", path);
            let symlink_src = fs::canonicalize(&cfg_dir)?;
            std::os::unix::fs::symlink(symlink_src, symlink_target)?;
        } else {
            add("etc")?;
            add("etc/toolchains")?;
            let cfg_dir_items = vec!["jjs.toml", "toolchains", "contest.toml"]
                .iter()
                .map(|x| format!("{}/{}", cfg_dir, x))
                .collect();
            fs_extra::copy_items(
                &cfg_dir_items,
                format!("{}/etc", path),
                &fs_extra::dir::CopyOptions::new(),
            )?;
        }
    }
    Ok(())
}
