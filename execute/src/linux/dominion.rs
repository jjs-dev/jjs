use crate::{
    Dominion, DominionOptions,
    linux::{Pid, LINUX_DOMINION_SANITY_CHECK_ID, allocate_heap_variable},
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    //mem::size_of,
};

use field_offset::offset_of;

#[derive(Debug)]
#[repr(C)]
pub struct LinuxDominion {
    cgroup_id: String,
    sanity_tag: usize,
}

impl Dominion for LinuxDominion {}

const ID_CHARS: &[u8] = b"qwertyuiopasdfghjklzxcvbnm1234567890";
const ID_SIZE: usize = 8;

fn gen_id() -> String {
    use rand::Rng;
    let mut gen = rand::thread_rng();
    let mut out = Vec::new();
    for _i in 0..ID_SIZE {
        let ch = gen.choose(ID_CHARS).cloned().unwrap();
        out.push(ch);
    }
    String::from_utf8_lossy(&out[..]).to_string()
}

impl LinuxDominion {
    pub fn sanity_check(&mut self) -> usize {
        self.sanity_tag
    }

    fn get_path_for_subsystem(&self, subsys_name: &str) -> String {
        format!("/sys/fs/cgroup/{}/jjs/g-{}", subsys_name, &self.cgroup_id)
    }

    pub fn create(options: DominionOptions) -> *mut LinuxDominion {
        let cgroup_id = gen_id();
        let cgroup_path = format!("/sys/fs/cgroup/pids/jjs/g-{}", &cgroup_id);
        fs::create_dir_all(&cgroup_path).unwrap();

        //configure pids subsystem
        fs::write(format!("{}/pids.max", &cgroup_path),
                  format!("{}", options.max_alive_process_count)).unwrap();

        let dmem = allocate_heap_variable::<LinuxDominion>();
        unsafe {
            let d = dmem.as_mut().unwrap();
            d.sanity_tag = LINUX_DOMINION_SANITY_CHECK_ID;

            let cgroup_ptr = offset_of!(LinuxDominion => cgroup_id).apply_ptr(dmem);
            let cgroup_ptr = cgroup_ptr as *mut String;
            std::ptr::write(cgroup_ptr, cgroup_id);

            //d.cgroup_id = cgroup_id;
        }
        dmem
    }

    pub fn add_process(&mut self, pid: Pid) {
        let tasks_file_path = format!("{}/tasks", self.get_path_for_subsystem("pids"));
        let mut f = OpenOptions::new()
            .append(true)
            .open(tasks_file_path)
            .unwrap();
        write!(f, "{}\n", pid);
    }
}