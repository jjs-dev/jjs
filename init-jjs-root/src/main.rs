use std::{env, fs, process};

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} path/to/sysroot", args[0]);
        process::exit(1);
    }

    let path = &args[1];
    {
        let mut dir = match fs::read_dir(path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("Couldn't read dir: {:?}", e);
                process::exit(2);
            }
        };
        if dir.next().is_some() {
            eprintln!("Specified dir is not empty");
            process::exit(3);
        }
    }

    let add = |name: &str| {
        let p = format!("{}/{}", path, name);
        match fs::create_dir(p) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Couldn't create {}: {:?}", name, e);
                process::exit(4);
            }
        }
    };

    add("var");
    add("var/jjs");
    add("var/jjs/submits");
    add("var/jjs/build");
    add("bin");
    add("lib");
    add("etc");
    add("etc/jjs");
    add("tmp");
}
