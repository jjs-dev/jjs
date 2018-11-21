use crate::{
    linux::{
        allocate_heap_variable, allocate_memory, err_exit, setup_pipe, Pid,
        LINUX_DOMINION_SANITY_CHECK_ID,
    },
    Dominion, DominionOptions,
};
use field_offset::offset_of;
use rand::seq::SliceRandom;
use std::{
    ffi::CString,
    fs::{self, OpenOptions},
    io::Write,
    process::{Command, Stdio},
    //mem::size_of,
    ptr,
};

#[derive(Debug)]
#[repr(C)]
struct NsInfo {
    mount: i32,
    pids: i32,
}

struct FillNsInfoArg {
    ns_info: NsInfo,
    done_fd: i32,
}

//clone() target func
fn fill_ns_info(arg: *mut _) {
    unsafe {
        let arg = &mut *(arg as *mut FillNsInfoArg);
        let nsi = &mut arg.ns_info;
        {
            let mp = CString::from("/proc/self/ns/mnt");
            nsi.mount = libc::open(mp.as_ptr());
        }
        {
            let pp = CString::from("/proc/self/pids");
            nsi.pids = libc::open()
        }
        let ready_msg = CString::from("d");
        libc::write(arg.done_fd, ready_msg.as_ptr(), 1);
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct LinuxDominion {
    cgroup_id: String,
    sanity_tag: u64,
    options: DominionOptions,
    user_id: u64,
    ns_info: NsInfo,
}

impl Dominion for LinuxDominion {}

const ID_CHARS: &[u8] = b"qwertyuiopasdfghjklzxcvbnm1234567890";
const ID_SIZE: usize = 8;

fn gen_id() -> String {
    let mut gen = rand::thread_rng();
    let mut out = Vec::new();
    for _i in 0..ID_SIZE {
        let ch = ID_CHARS.choose(&mut gen).unwrap().clone();
        out.push(ch);
    }
    String::from_utf8_lossy(&out[..]).to_string()
}

//TODO extract to crate
#[allow(dead_code)]
fn dev_log(s: &str) {
    let c = CString::new(s).unwrap();
    unsafe {
        libc::write(-1, c.as_ptr() as *const libc::c_void, s.len());
    }
}

const MINION_GROUP_NAME: &str = "minion_sandbox";

fn get_group_id() -> u64 {
    //Firstly, we create the group
    //If it already exists, we silently ignore error
    let mut groupadd = Command::new("groupadd")
        .arg(MINION_GROUP_NAME)
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    groupadd.wait().unwrap();
    //now we retrieve group id
    let group_description = Command::new("getent")
        .arg("group")
        .arg(MINION_GROUP_NAME)
        .output()
        .unwrap();
    let items = String::from_utf8_lossy(&group_description.stdout).to_string();
    let items: Vec<_> = items.split(':').collect();
    assert_eq!(items[0], "minion_sandbox");
    items[2].parse().unwrap()
}

fn allocate_user(name: &str) -> u64 {
    let name = format!("minion_sandbox_{}", name);
    get_group_id();
    Command::new("useradd")
        .args(&["--no-user-group", "--no-create-home"])
        .arg("--gid")
        .arg(MINION_GROUP_NAME)
        .arg(&name)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    let res = Command::new("id")
        .arg("-u")
        .arg(&name)
        .output()
        .unwrap()
        .stdout;
    let mut res = String::from_utf8_lossy(&res).to_string();
    res = res.trim().into();
    res.parse().unwrap()
}

pub struct DesiredAccess {
    pub r: bool,
    pub w: bool,
    pub x: bool,
}

impl LinuxDominion {
    pub(crate) fn sanity_check(&mut self) -> u64 {
        self.sanity_tag
    }

    fn get_path_for_subsystem(subsys_name: &str, cgroup_id: &str) -> String {
        format!("/sys/fs/cgroup/{}/jjs/g-{}", subsys_name, cgroup_id)
    }

    pub(crate) fn create(options: DominionOptions) -> *mut LinuxDominion {
        unsafe {
            let cgroup_id = gen_id();

            //configure pids subsystem
            let pids_cgroup_path = LinuxDominion::get_path_for_subsystem("pids", &cgroup_id);
            fs::create_dir_all(&pids_cgroup_path).unwrap();

            fs::write(
                format!("{}/pids.max", &pids_cgroup_path),
                format!("{}", options.max_alive_process_count),
            )
            .unwrap();

            //configure memory subsystem
            let mem_cgroup_path = LinuxDominion::get_path_for_subsystem("memory", &cgroup_id);

            fs::create_dir_all(&mem_cgroup_path).unwrap();
            fs::write(format!("{}/memory.swappiness", &mem_cgroup_path), "0").unwrap();

            fs::write(
                format!("{}/memory.limit_in_bytes", &mem_cgroup_path),
                format!("{}", options.memory_limit),
            )
            .unwrap();

            let user_id = allocate_user(&cgroup_id);

            let dmem = allocate_heap_variable::<LinuxDominion>();

            let d = dmem.as_mut().unwrap();
            (*d).sanity_tag = LINUX_DOMINION_SANITY_CHECK_ID;

            (*d).user_id = user_id;

            let cgroup_ptr = offset_of!(LinuxDominion => cgroup_id).apply_ptr(dmem);
            let cgroup_ptr = cgroup_ptr as *mut _;
            ptr::write(cgroup_ptr, cgroup_id);

            let options_ptr = offset_of!(LinuxDominion => options).apply_ptr(dmem);
            let options_ptr = options_ptr as *mut _;
            ptr::write(options_ptr, options.clone());

            //now we setup ns_info
            let (mut done_r, mut done_w);
            setup_pipe(&mut done_r, &mut done_w);
            let child_arg: *mut FillNsInfoArg = allocate_heap_variable();
            (*child_arg).done_fd = done_w;
            let child_stack = allocate_memory(1024 * 1024);
            libc::clone(
                fill_ns_info,
                child_stack,
                libc::CLONE_VM | libc::CLONE_NEWNS | libc::CLONE_NEWPID,
                child_arg as *mut _,
            );

            dmem
        }
    }

    fn add_to_subsys(&mut self, pid: Pid, subsys: &str) {
        let cgroup_path = LinuxDominion::get_path_for_subsystem(subsys, &self.cgroup_id);
        let tasks_file_path = format!("{}/tasks", cgroup_path);
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(tasks_file_path)
            .unwrap();
        write!(f, "{}", pid).unwrap();
    }

    pub(crate) fn add_process(&mut self, pid: Pid) {
        for subsys in vec!["pids", "memory"] {
            self.add_to_subsys(pid, subsys);
        }
    }

    pub(crate) fn dir(&self) -> String {
        let res = self.options.isolation_root.clone();
        let res = res.to_str();
        let res = res.unwrap();
        let res = String::from(res);
        res
    }

    fn get_mount_target(&self, suf: &str) -> String {
        let mut suf: String = suf.into();
        if suf.starts_with('/') {
            suf = suf[1..].into();
        }
        if suf.ends_with('/') {
            suf.pop();
        }
        let mut dir = self.dir();
        if dir.ends_with('/') {
            dir.pop();
        }
        return format!("{}/{}", dir, suf);
    }

    #[allow(unused_must_use)]
    fn expose_dir(&self, system_path: &str, alias_path: &str, _access: DesiredAccess) {
        let bind_target = self.get_mount_target(alias_path);
        fs::create_dir_all(&bind_target); //TODO check error
        let bind_target = CString::new(bind_target).unwrap();
        let bind_src = CString::new(system_path).unwrap();
        unsafe {
            let mnt_res = libc::mount(
                bind_src.as_ptr(),
                bind_target.clone().as_ptr(),
                ptr::null(),
                libc::MS_BIND,
                ptr::null(),
            );
            if mnt_res == -1 {
                err_exit("mount");
            }

            let rem_ret = libc::mount(
                ptr::null(),
                bind_target.clone().as_ptr(),
                ptr::null(),
                libc::MS_BIND | libc::MS_REMOUNT | libc::MS_RDONLY,
                ptr::null(),
            );
            if rem_ret == -1 {
                err_exit("mount");
            }
        }
    }

    pub(crate) fn expose_dirs(&self) {
        //mount --bind
        for x in &self.options.exposed_paths {
            self.expose_dir(
                &x.src,
                &x.dest,
                DesiredAccess {
                    r: x.allow_read,
                    w: x.allow_write,
                    x: x.allow_execute,
                },
            )
        }
    }

    pub(crate) fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

impl Drop for LinuxDominion {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        //remove cgroups
        for subsys in &["pids, memory"] {
            fs::remove_dir(LinuxDominion::get_path_for_subsystem(
                subsys,
                &self.cgroup_id,
            ));
        }
        //remove user
        Command::new("userdel")
            .arg(&format!("minion_sandbox_{}", &self.cgroup_id))
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        //unmount exposed dirs
        for x in &self.options.exposed_paths {
            let bind_target = self.get_mount_target(&x.dest);
            let bind_target = CString::new(bind_target).unwrap();
            unsafe {
                //libc::umount2(bind_target.as_ptr(), libc::MNT_DETACH);
            }
        }
    }
}
