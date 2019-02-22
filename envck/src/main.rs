use std::{fs, io::ErrorKind, process::exit};

fn main() {
    println!("checking user");
    {
        let uid = unsafe { libc::getuid() };
        if uid != 0 {
            eprintln!("ERROR: must be run as root");
            exit(1);
        }
    }
    println!("checking cgroupfs");
    {
        let items = match fs::read_dir("/sys/fs/cgroup") {
            Ok(x) => x,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    eprintln!("ERROR: /sys/fs/cgroup not found");
                    exit(1);
                }
                _ => {
                    eprintln!("ERROR: {:?}", e);
                    exit(1);
                }
            },
        };
        let items: Vec<_> = items
            .map(|x| x.unwrap().file_name().into_string().unwrap())
            .collect();
        for subsys in ["pids", "cpuacct", "memory"].iter() {
            if !items.contains(&String::from(*subsys)) {
                eprintln!("ERROR: subsystem {} not found", subsys);
                exit(1);
            }
        }
    }
}
