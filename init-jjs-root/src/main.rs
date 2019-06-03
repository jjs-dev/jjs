use std::{fs, process};
use structopt::StructOpt;
#[derive(StructOpt)]
struct CliArgs {
    sysroot_dir: String,
    config_dir: String,
    #[structopt(long = "symlink-config")]
    symlink_config: bool,
}
fn main() {
    let args: CliArgs = CliArgs::from_args();

    let path = &args.sysroot_dir;
    let cfg_dir_path = &args.config_dir;
    {
        let mut dir = match fs::read_dir(path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("Couldn't read dir: {:?}", e);
                process::exit(1);
            }
        };
        if dir.next().is_some() {
            eprintln!("Specified dir is not empty");
            process::exit(1);
        }
    }

    let add = |name: &str| {
        let p = format!("{}/{}", path, name);
        match fs::create_dir(p) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Couldn't create {}: {:?}", name, e);
                process::exit(1);
            }
        }
    };

    add("var");
    add("var/submissions");
    add("var/problems");
    add("opt");
    add("opt/bin");
    add("opt/lib64");
    add("opt/lib");
    add("opt/etc");
    if args.symlink_config {
        let symlink_target = format!("{}/etc", path);
        let symlink_src = fs::canonicalize(&args.config_dir).unwrap();
        std::os::unix::fs::symlink(symlink_src, symlink_target).expect("Coudln't symlink config");
    } else {
        add("etc");
        add("etc/toolchains");
        let cfg_dir_items = vec!["/jjs.toml", "toolchains"]
            .iter()
            .map(|x| format!("{}/{}", cfg_dir_path, x))
            .collect();
        fs_extra::copy_items(
            &cfg_dir_items,
            format!("{}/etc", path),
            &fs_extra::dir::CopyOptions::new(),
        )
        .expect("Couldn't copy config files");
    }
}
