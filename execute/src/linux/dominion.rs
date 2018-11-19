use crate::{
    linux::{allocate_heap_variable, err_exit, Pid, LINUX_DOMINION_SANITY_CHECK_ID},
    Dominion, DominionOptions,
};
use field_offset::offset_of;
use rand::seq::SliceRandom;
use std::{
    ffi::CString,
    fs::{self, OpenOptions},
    io::Write,
    ptr,
    process::{Command},
    //mem::size_of,
};

#[derive(Debug)]
#[repr(C)]
pub struct LinuxDominion {
    cgroup_id: String,
    sanity_tag: u64,
    options: DominionOptions,
    user_id: u64,
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
    let res = String::from_utf8_lossy(&res).to_string();
    res.parse().unwrap()
}

impl LinuxDominion {
    pub fn sanity_check(&mut self) -> u64 {
        self.sanity_tag
    }

    fn get_path_for_subsystem(subsys_name: &str, cgroup_id: &str) -> String {
        format!("/sys/fs/cgroup/{}/jjs/g-{}", subsys_name, cgroup_id)
    }

    pub fn create(options: DominionOptions) -> *mut LinuxDominion {
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
        unsafe {
            let d = dmem.as_mut().unwrap();
            (*d).sanity_tag = LINUX_DOMINION_SANITY_CHECK_ID;

            (*d).user_id = user_id;

            let cgroup_ptr = offset_of!(LinuxDominion => cgroup_id).apply_ptr(dmem);
            let cgroup_ptr = cgroup_ptr as *mut _;
            ptr::write(cgroup_ptr, cgroup_id);

            let options_ptr = offset_of!(LinuxDominion => options).apply_ptr(dmem);
            let options_ptr = options_ptr as *mut _;
            ptr::write(options_ptr, options);
        }
        dmem
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

    pub fn add_process(&mut self, pid: Pid) {
        for subsys in vec!["pids", "memory"] {
            self.add_to_subsys(pid, subsys);
        }
    }

    pub fn dir(&self) -> String {
        let res = self.options.isolation_root.clone();
        let res = res.to_str();
        let res = res.unwrap();
        let res = String::from(res);
        res
    }

    pub fn expose_dir(&self, system_path: &str, alias_path: &str) {
        let bind_target = format!("{}/{}", &self.dir(), alias_path);
        let bind_target = CString::new(bind_target).unwrap();
        let bind_src = CString::new(system_path).unwrap();
        let ign = CString::new("IGNORED").unwrap();
        unsafe {
            if -1 == libc::mount(
                bind_src.as_ptr(),
                bind_target.as_ptr(),
                ign.clone().as_ptr(),
                libc::MS_BIND,
                ign.clone().as_ptr() as *const _,
            ) {
                err_exit("LinuxDominion::expose_dir", "mount");
            }
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
}
