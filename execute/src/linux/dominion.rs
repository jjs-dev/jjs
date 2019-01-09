use crate::{
    linux::{
        jail_common,
        util::{allocate_heap_variable, err_exit, Handle, Sock},
    },
    Dominion, DominionOptions,
};
use field_offset::offset_of;
use std::{ffi::CString, fs, ptr};

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct NsInfo {
    pid: Handle,
    user: Handle,
    net: Handle,
}

#[repr(C)]
struct FillNsInfoArg {
    ns_info: NsInfo,
    sock: Sock,
    user_id: i64,
}

#[derive(Debug)]
#[repr(C)]
pub struct LinuxDominion {
    id: String,
    options: DominionOptions,
    jobserver_sock: Sock,
}

impl Dominion for LinuxDominion {}

//TODO extract to crate
#[allow(dead_code)]
fn dev_log(s: &str) {
    let c = CString::new(s).unwrap();
    unsafe {
        libc::write(-1, c.as_ptr() as *const libc::c_void, s.len());
    }
}

fn allocate_user_id() -> u64 {
    use rand::Rng;
    //TODO make parameters
    rand::thread_rng().gen_range(1_000_000, 2_000_000)
}

pub struct DesiredAccess {
    pub r: bool,
    pub w: bool,
    pub x: bool,
}

const CHILD_STACK_SIZE: usize = 1024 * 1024;

impl LinuxDominion {
    pub(crate) unsafe fn create(options: DominionOptions) -> *mut LinuxDominion {
        let dmem = allocate_heap_variable::<LinuxDominion>();

        let d = dmem.as_mut().unwrap();

        let options_ptr = offset_of!(LinuxDominion => options).apply_ptr(dmem);
        let options_ptr = options_ptr as *mut _;
        ptr::write(options_ptr, options.clone());

        dmem
    }

    pub(crate) fn dir(&self) -> String {
        let res = self.options.isolation_root.clone();
        let res = res.to_str();
        let res = res.unwrap();
        String::from(res)
    }
}

impl Drop for LinuxDominion {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        //remove cgroups
        for subsys in &["pids, memory"] {
            fs::remove_dir(jail_common::get_path_for_subsystem(subsys, &self.id));
        }

        let do_umount = |inner_path: &str| {
            let mount_path = format!(
                "{}/{}",
                &self.options.isolation_root.to_str().unwrap(),
                inner_path
            );
            let mount_path = CString::new(mount_path.as_str()).unwrap();
            unsafe {
                if libc::umount2(mount_path.as_ptr(), libc::MNT_DETACH) == -1 {
                    err_exit("umount2");
                }
            }
        };

        do_umount("/proc");
        for x in &self.options.exposed_paths {
            do_umount(x.dest.as_str());
        }
    }
}
