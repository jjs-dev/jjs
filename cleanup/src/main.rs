use std::{env, ffi::CString, fs};

extern crate structopt;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Argv {
    #[structopt(long = "cgroupfs", short = "c", default_value = "/sys/fs/cgroup")]
    cgroupfs: String,

    #[structopt(long = "root", short = "r")]
    root: String,

    #[structopt(long = "jail", short = "j")]
    jail_id: String,
}

fn main() {
    let argv: Argv = Argv::from_args();
    println!("----> Procfs");
    let procfs_path = format!("{}/proc", &argv.root);
    let self_exe = format!("{}/self/exe", &procfs_path);
    let should_unmount = match fs::File::open(self_exe) {
        Ok(_) => true,
        Err(_) => false,
    };
    if should_unmount {
        println!("Ok: procfs is not mounted");
    } else {
        println!("Unmounting");
        let procfs_path = CString::new(procfs_path).unwrap();
        unsafe {
            if libc::umount2(procfs_path.as_ptr(), libc::MNT_DETACH) == -1 {
                let err = nix::errno::errno();
                let err = nix::errno::from_i32(err);
                eprintln!("Error while unmounting procfs: {:?}", err);
            }
        }
        println!("done");
    }
    println!("----> Cgroups");
    for subsys in &["pids", "memory", "cpuacct"] {
        let path = format!("{}/{}/jjs/g-{}", &argv.cgroupfs, subsys, argv.jail_id);
        println!("deleting {}",&path);
        if let Err(e) = fs::remove_dir(path) {
            eprintln!("Error: {:?}", e);
        }
    }

}
