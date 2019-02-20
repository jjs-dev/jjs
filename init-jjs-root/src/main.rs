use std::{env, fs, process};

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} path/to/sysroot path/to/example-config", args[0]);
        process::exit(1);
    }

    let path = &args[1];
    let cfg_dir_path = &args[2];
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
    add("opt");
    add("opt/bin");
    add("opt/lib64");
    add("opt/lib");
    add("opt/etc");
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
