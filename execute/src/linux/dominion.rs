use crate::linux::util::{
    allocate_heap_variable, allocate_memory, err_exit, Pid, Sock, WaitMessage,
};
use crate::{linux::LINUX_DOMINION_SANITY_CHECK_ID, Dominion, DominionOptions};
use field_offset::offset_of;
use rand::seq::SliceRandom;
use std::{
    ffi::CString,
    fs::{self, OpenOptions},
    io::{self, Write},
    ptr,
};

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct NsInfo {
    pid: i32,
    user: i32,
}

#[repr(C)]
struct FillNsInfoArg {
    ns_info: NsInfo,
    sock: Sock,
    user_id: i64,
}

const WAIT_CODE_DOMINION_CREATED: u16 = 1;
const WAIT_CODE_PID_MAP_READY_FOR_SETUP: u16 = 2;
const WAIT_CODE_PID_MAP_CREATED: u16 = 3;

//clone() target func
extern "C" fn fill_ns_info(arg: *mut libc::c_void) -> i32 {
    unsafe {
        let arg = &mut *(arg as *mut FillNsInfoArg);
        let nsi = &mut arg.ns_info;
        {
            let pp = CString::new("/proc/self/ns/pid").unwrap();
            nsi.pid = libc::open(pp.as_ptr(), libc::O_RDONLY);
        }
        {
            let up = CString::new("/proc/self/ns/user").unwrap();
            nsi.user = libc::open(up.as_ptr(), libc::O_RDONLY);
        }
        //now we need to setup namespaces

        //mount procfs (it's required for FD closing to work)

        //init uid_map and gid_map (see user_namespaces(2))
        //for doing this, we will call our master process
        arg.sock
            .send(&WaitMessage::with_class(WAIT_CODE_PID_MAP_READY_FOR_SETUP));
        let resp = arg.sock.receive::<WaitMessage>();
        resp.check(WAIT_CODE_PID_MAP_CREATED).unwrap();

        arg.sock
            .send(&WaitMessage::with_class(WAIT_CODE_DOMINION_CREATED));

        0
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
        let ch = *(ID_CHARS.choose(&mut gen).unwrap());
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
    pub(crate) fn sanity_check(&mut self) -> u64 {
        self.sanity_tag
    }

    fn get_path_for_subsystem(subsys_name: &str, cgroup_id: &str) -> String {
        format!("/sys/fs/cgroup/{}/jjs/g-{}", subsys_name, cgroup_id)
    }

    fn fill_pid_gid_map_for_child(sock: &mut Sock, child_pid: i32, mapped_uid: i32) {
        {
            let wm: WaitMessage = sock.receive();
            wm.check(WAIT_CODE_PID_MAP_READY_FOR_SETUP);
        }
        //child will have uid=1 in its namespace, but some random and not-privileged in primary
        let mapping = format!("1 {} 1\n", mapped_uid);
        let uid_map_path = format!("/proc/{}/uid_map", child_pid);
        let gid_map_path = format!("/proc/{}/gid_map", child_pid);
        fs::write(&uid_map_path, mapping.as_str()).unwrap();
        fs::write(&gid_map_path, mapping.as_str()).unwrap();
        sock.send(&WaitMessage::with_class(WAIT_CODE_PID_MAP_CREATED));
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

            //mount procfs
            {
                let procfs_path = format!(
                    "{}/proc",
                    &options.isolation_root.to_str().unwrap().to_string()
                );
                match fs::create_dir(procfs_path.as_str()) {
                    Ok(_) => (),
                    Err(e) => match e.kind() {
                        io::ErrorKind::AlreadyExists => (),
                        _ => Err(e).unwrap(),
                    },
                }
                let proc = CString::new("proc").unwrap();
                let targ = CString::new(procfs_path.as_str()).unwrap();
                let mret = libc::mount(
                    proc.clone().as_ptr(),
                    targ.clone().as_ptr(),
                    proc.clone().as_ptr(),
                    0,
                    ptr::null(),
                );
                if -1 == mret {
                    err_exit("mount")
                }
            }

            let user_id = allocate_user_id();

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

            let child_arg: *mut FillNsInfoArg = allocate_heap_variable();
            (*child_arg).user_id = user_id as i64;

            let (mut sock, child_sock) = Sock::make_pair();

            let mut child_stack = allocate_memory(CHILD_STACK_SIZE);
            (*child_arg).sock = child_sock;
            //we must provide pointer to end of child_stack, because stack grows right-to-left
            child_stack = child_stack.add(CHILD_STACK_SIZE);
            let child_pid = libc::clone(
                fill_ns_info,
                child_stack as *mut _,
                libc::CLONE_VM | libc::CLONE_NEWPID | libc::CLONE_NEWUSER | libc::CLONE_FILES,
                child_arg as *mut _,
            );
            if child_pid == -1 {
                err_exit("clone");
            }
            Self::fill_pid_gid_map_for_child(&mut sock, child_pid, user_id as i32);
            {
                let wm: WaitMessage = sock.receive();
                wm.check(WAIT_CODE_DOMINION_CREATED).unwrap();
            }
            (*dmem).ns_info = (*child_arg).ns_info;

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
        dev_log("add_process");
        for subsys in &["pids", "memory"] {
            self.add_to_subsys(pid, subsys);
        }
    }

    pub(crate) fn dir(&self) -> String {
        let res = self.options.isolation_root.clone();
        let res = res.to_str();
        let res = res.unwrap();
        String::from(res)
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

    pub(crate) fn enter(&self) {
        unsafe {
            if libc::setns(self.ns_info.pid, libc::CLONE_NEWPID) == -1 {
                err_exit("setns");
            }
            if libc::setns(self.ns_info.user, libc::CLONE_NEWUSER) == -1 {
                err_exit("setns");
            }
        }
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
        //unmount all
        let fres = unsafe { libc::fork() };
        if fres == -1 {
            err_exit("fork");
        }
        if fres == 0 {
            self.enter();
            let mount_info = fs::read_to_string("/proc/self/mounts").unwrap();
            let mount_info = mount_info.split('\n');
            for line in mount_info {
                let line = line.trim();
                let parts: Vec<String> = line.split(' ').map(|x| x.to_string()).collect();
                let mount_path = CString::new(parts[1].as_str()).unwrap();
                unsafe {
                    if libc::umount2(mount_path.as_ptr(), libc::MNT_DETACH) == -1 {
                        err_exit("umount2");
                    }
                }
            }
            unsafe { libc::exit(0) }
        }
    }
}
