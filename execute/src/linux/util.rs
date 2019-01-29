use libc::{self, c_char, c_int, c_void};
use std::{
    ffi::CString,
    fmt::{self, Display, Formatter},
    io, mem,
    os::unix::io::RawFd,
    ptr,
};
use tiny_nix_ipc::{self, Socket};

pub type Handle = RawFd;
pub type Pid = libc::pid_t;
pub type ExitCode = c_int;
pub type Uid = libc::uid_t;

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

#[derive(Debug)]
pub struct WaitMessage {
    class: u16,
}

impl Display for WaitMessage {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.class)
    }
}

impl WaitMessage {
    #[must_use]
    pub fn check(&self, oth_class: u16) -> Option<()> {
        if self.class == oth_class {
            Some(())
        } else {
            None
        }
    }
    pub fn new(class: u16) -> WaitMessage {
        WaitMessage { class }
    }
}

fn sock_lock(sock: &mut Socket, expected_class: u16) {
    let wm = sock.recv_struct::<WaitMessage, [RawFd; 0]>().unwrap();
    wm.0.check(expected_class).unwrap()
}

fn sock_wake(sock: &mut Socket, wake_class: u16) {
    let wm = WaitMessage::new(wake_class);
    sock.send_struct(&wm, None).unwrap();
}

pub trait IpcSocketExt {
    type Err;
    fn lock(&mut self, expected_class: u16);
    fn wake(&mut self, wake_class: u16);

    fn send<T: serde::ser::Serialize>(&mut self, data: &T) -> Result<(), Self::Err>;
    fn recv<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Self::Err>;
}

impl IpcSocketExt for Socket {
    type Err = tiny_nix_ipc::errors::Error;

    fn lock(&mut self, expected_class: u16) {
        sock_lock(self, expected_class)
    }

    fn wake(&mut self, wake_class: u16) {
        sock_wake(self, wake_class)
    }

    fn send<T: serde::ser::Serialize>(&mut self, data: &T) -> Result<(), Self::Err> {
        self.send_cbor(data, None).map(|_num_read| ())
    }

    fn recv<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Self::Err> {
        self.recv_cbor::<T, [RawFd; 0]>(4096).map(|(x, _fds)| x)
    }
}

pub trait IgnoreExt: Sized {
    #[allow(unused_must_use)]
    fn ignore(self) {
        //empty
    }
}

impl<T, E> IgnoreExt for Result<T, E> {}

pub fn duplicate_string(arg: &str) -> *mut c_char {
    unsafe {
        let cstr = CString::new(arg).unwrap();
        let strptr = cstr.as_ptr();
        libc::strdup(strptr)
    }
}

#[derive(Copy, Clone)]
pub struct StraceLogger;

#[allow(dead_code)]
pub fn strace_logger() -> StraceLogger {
    StraceLogger
}

impl io::Write for StraceLogger {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let invalid_fd: Handle = -1;
        unsafe {
            libc::write(invalid_fd, buf.as_ptr() as *const c_void, buf.len());
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        //empty
        Ok(())
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
