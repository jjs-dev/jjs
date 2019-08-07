use std::{
    fs,
    io::ErrorKind,
    process::{exit, Command},
};

fn main() {
    println!("checking user");
    {
        let uid = unsafe { libc::getuid() };
        if uid != 0 {
            eprintln!("ERROR: must be run as root, but uid is {}", uid);
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
    println!("Checking kernel version");
    {
        let out = Command::new("uname")
            .arg("-r")
            .output()
            .expect("failed run 'uname -r'");
        if !out.status.success() {
            let code = out
                .status
                .code()
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "<unknown>".to_string());
            eprintln!(
                "error:\nexit-code={}\n---output---\n{}\n---stderr---\n{}",
                code,
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let version = String::from_utf8(out.stdout).expect("failed decode uname output");
        let version = version.trim();
        let version = semver::Version::parse(&version).unwrap_or_else(|err| {
            eprintln!("failed parse '{}': {}", version, err);
            exit(1);
        });
        // TODO: relax
        let min_version = semver::Version {
            major: 5,
            minor: 0,
            patch: 0,
            pre: vec![],
            build: vec![],
        };
        if version < min_version {
            eprintln!("error: Linux Kernel version {} is unsupported. Minimal supported version is currently {}", version, min_version);
        }
    }
    println!("OK");
}
