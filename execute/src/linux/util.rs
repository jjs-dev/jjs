use libc::{self, c_char, c_int};
use std::{ffi::CString, mem, ptr, str::FromStr};

pub type Handle = c_int;
pub type Pid = libc::pid_t;
pub type ExitCode = c_int;

pub fn err_exit(syscall_name: &str) -> ! {
    unsafe {
        let e = errno::errno();
        eprintln!("{}() failed with error {}: {}", syscall_name, e.0, e);
        if libc::getpid() != 1 {
            panic!("syscall error (msg upper)")
        } else {
            libc::exit(libc::EXIT_FAILURE);
        }
    }
}

//allow dead_code because this function is only intended for debugging
#[allow(dead_code)]
pub fn dev_log(msg: &str) {
    //TODO throw error in Release build
    let msg = format!("dev_log: {}", msg);
    let msg_len = msg.len();
    let msg = CString::new(msg.as_str()).unwrap();
    unsafe {
        libc::write(-1, msg.as_ptr() as *const _, msg_len);
    }
}
pub struct Sock {
    fd: i32,
}

pub trait Message: ToString + FromStr {}

impl Sock {
    pub fn send<T: Message>(&mut self, value: &T) {
        let res = value.to_string();
        let buf = res.as_bytes();
        let buf_len = buf.len();
        unsafe {
            if libc::write(self.fd, buf.as_ptr() as *const _, buf_len) == -1 {
                err_exit("write");
            }
        }
    }

    pub fn receive<T: Message>(&mut self) -> T {
        let mut buf = [0; 4096];
        let buf_len = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut _, buf.len()) };
        if buf_len == -1 {
            err_exit("read");
        }
        let buf = String::from_utf8_lossy(&buf[..buf_len as usize]).to_string();
        match T::from_str(&buf) {
            Ok(x) => x,
            Err(_e) => panic!("protocol error"),
        }
    }

    pub fn make_pair() -> (Sock, Sock) {
        let mut fds = [0; 2];
        unsafe {
            if libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) == -1 {
                err_exit("socketpair");
            }
        }
        let s1 = Sock { fd: fds[0] };
        let s2 = Sock { fd: fds[1] };
        (s1, s2)
    }
}

pub struct WaitMessage {
    class: u16,
}

impl WaitMessage {
    pub fn check(&self, oth_class: u16) -> Option<()> {
        if self.class == oth_class {
            Some(())
        } else {
            None
        }
    }
    pub fn with_class(class: u16) -> WaitMessage {
        WaitMessage { class }
    }
}

impl ToString for WaitMessage {
    fn to_string(&self) -> String {
        format!("{}", self.class)
    }
}

impl FromStr for WaitMessage {
    type Err = <u16 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        Ok(Self { class: s.parse()? })
    }
}

impl Message for WaitMessage {}

pub fn duplicate_string(arg: &str) -> *mut c_char {
    unsafe {
        let cstr = CString::new(arg).unwrap();
        let strptr = cstr.as_ptr();
        libc::strdup(strptr)
    }
}

pub fn allocate_memory(num: usize) -> *mut c_char {
    unsafe {
        let p = libc::malloc(num) as *mut c_char;
        if p as usize == 0 {
            panic!("OutOfMemory: malloc returned null");
        }
        ptr::write_bytes(p, 0xDC, num);
        p
    }
}

pub fn allocate_heap_variable<T>() -> *mut T {
    allocate_memory(mem::size_of::<T>()) as *mut T
}

pub fn setup_pipe(read_end: &mut Handle, write_end: &mut Handle) {
    unsafe {
        let mut ends = [0 as Handle; 2];
        let ret = libc::pipe(ends.as_mut_ptr());
        if ret == -1 {
            err_exit("pipe");
        }
        *read_end = ends[0];
        *write_end = ends[1];
    }
}
