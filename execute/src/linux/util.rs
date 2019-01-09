use failure::Fail;
use libc::{self, c_char, c_int, c_void};
use std::{
    alloc, error,
    ffi::CString,
    fmt::{self, Debug, Display, Formatter},
    io, mem, ptr,
    str::FromStr,
};

pub type Handle = c_int;
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
pub struct Sock {
    fd: i32,
}

pub trait Message: Sized {
    type Error;
    unsafe fn send(data: &Self, socket: Handle) -> Result<(), Self::Error>;
    unsafe fn receive(socket: Handle) -> Result<Self, Self::Error>;
}

pub trait SimpleMessage: 'static + Debug + Display + ToString + FromStr
where
    <Self as FromStr>::Err: Send + Sync + error::Error + Debug,
{
}

#[derive(Debug, Fail)]
pub enum SimpleMessageError<T: SimpleMessage>
where
    <T as FromStr>::Err: Send + Sync + Debug + error::Error,
{
    #[fail(display = "I/O error: {}", _0)]
    Io(#[cause] io::Error),

    #[fail(display = "Parsing error: {}", _0)]
    Parse(#[cause] <T as FromStr>::Err),

    #[fail(display = "Protocol error")]
    Protocol,
}

impl<T: SimpleMessage> From<io::Error> for SimpleMessageError<T>
where
    <T as FromStr>::Err: Send + Sync + Debug + error::Error,
{
    fn from(x: io::Error) -> Self {
        SimpleMessageError::Io(x)
    }
}

impl<T: SimpleMessage> Message for T
where
    <T as FromStr>::Err: Send + Sync + Debug + error::Error,
{
    type Error = SimpleMessageError<T>;
    unsafe fn send(data: &Self, socket: Handle) -> Result<(), Self::Error> {
        let data = data.to_string();
        let data = data.as_bytes();
        let data_len = data.len();
        let data_len = format!("{}", data_len);
        let data_len = data_len.as_bytes();
        let mut pad_data_len = [0; 8];
        pad_data_len[..data_len.len()].clone_from_slice(&data_len[..]);
        //for i in 0..data_len.len() {
        //   pad_data_len[i] = data_len[i];
        //}
        if libc::write(
            socket,
            data_len.as_ptr() as *const c_void,
            pad_data_len.len(),
        ) == -1
        {
            Err(io::Error::last_os_error())?;
        }
        if libc::write(socket, data.as_ptr() as *const c_void, data.len()) == -1 {
            Err(io::Error::last_os_error())?;
        }
        Ok(())
    }

    unsafe fn receive(socket: Handle) -> Result<T, Self::Error> {
        //TODO: check max message size
        let mut data_len = [0; 8];
        let num_read = libc::read(socket, data_len.as_mut_ptr() as *mut c_void, data_len.len());
        if num_read == -1 {
            Err(io::Error::last_os_error())?;
        }
        let num_read = num_read as usize;
        let mut data_size = 0;
        for c in data_len.iter().cloned().take(num_read) {
            if c < b'0' || c > b'9' {
                return Err(SimpleMessageError::Protocol);
            }
            data_size *= 10;
            data_size += (c - b'0') as usize;
        }
        let mut data = Vec::new();
        data.resize(data_size, 0);
        let res = libc::read(socket, data.as_mut_ptr() as *mut c_void, data_size);
        if res == -1 {
            Err(io::Error::last_os_error())?;
        }
        let res = res as usize;
        if res < data_size {
            return Err(SimpleMessageError::Protocol);
        }
        let data = String::from_utf8_lossy(&data).to_string();
        let value = match data.parse() {
            Ok(x) => x,
            Err(e) => return Err(SimpleMessageError::Parse(e)),
        };
        Ok(value)
    }
}

///allows handle passing
pub struct HandleParcel(Handle);

impl HandleParcel {
    pub fn make(h: Handle) -> HandleParcel {
        HandleParcel(h)
    }

    pub fn into_inner(self) -> Handle {
        self.0
    }

    const SANITY_BYTE: u8 = 0x2F; //just magic number
}
#[derive(Debug, Fail)]
pub enum HandleParcelError {
    #[fail(display = "I/O error: {}", _0)]
    Io(#[cause] io::Error),

    #[fail(display = "protocol error")]
    Protocol,
}

impl From<io::Error> for HandleParcelError {
    fn from(e: io::Error) -> HandleParcelError {
        HandleParcelError::Io(e)
    }
}

impl Message for HandleParcel {
    type Error = HandleParcelError;

    unsafe fn send(data: &Self, socket: Handle) -> Result<(), HandleParcelError> {
        let pmsg: *mut libc::msghdr = allocate_heap_variable();
        let msg = &mut *pmsg;
        msg.msg_name = ptr::null_mut();
        msg.msg_namelen = 0;
        let iobuf = allocate_memory(1);
        *iobuf = HandleParcel::SANITY_BYTE as i8;
        let piov: *mut libc::iovec = allocate_heap_variable();
        {
            let iov = &mut *piov;
            iov.iov_base = iobuf as *mut c_void;
            iov.iov_len = 1;
        }
        msg.msg_iov = piov;
        msg.msg_iovlen = 0;

        let anc_buf_data = [data.0 /*it is handle we want to pass*/];
        let anc_buf_size = libc::CMSG_SPACE(anc_buf_data.len() as u32);
        msg.msg_controllen = anc_buf_size as _;
        let anc_layout =
            alloc::Layout::from_size_align(anc_buf_size as usize, mem::size_of::<libc::cmsghdr>())
                .unwrap();
        let anc_buf = alloc::alloc(anc_layout);
        msg.msg_control = anc_buf as *mut c_void;
        let pcmsg: *mut libc::cmsghdr;
        pcmsg = libc::CMSG_FIRSTHDR(pmsg);
        let cmsg = &mut *pcmsg;
        cmsg.cmsg_level = libc::SOL_SOCKET;
        cmsg.cmsg_type = libc::SCM_RIGHTS;
        cmsg.cmsg_len = mem::size_of::<Handle>();
        let cmsg_payload = libc::CMSG_DATA(pcmsg);
        ptr::copy(
            anc_buf_data.as_ptr() as *const u8,
            cmsg_payload as *mut u8,
            mem::size_of::<Handle>(),
        );
        if libc::sendmsg(socket, pmsg, 0) == -1 {
            Err(io::Error::last_os_error())?;
        }
        Ok(())
    }

    unsafe fn receive(socket: Handle) -> Result<Self, HandleParcelError> {
        let mut msg: libc::msghdr = mem::zeroed();

        //we don't want receive any addresses
        msg.msg_name = ptr::null_mut();
        msg.msg_namelen = 0;
        //now we setup iov
        let mut iov: libc::iovec = mem::zeroed();
        let mut recv_buf = [0 as c_char; 1];
        iov.iov_base = recv_buf.as_mut_ptr() as *mut c_void;
        iov.iov_len = 1;

        msg.msg_iov = (&mut iov) as *mut libc::iovec;
        msg.msg_iovlen = 1;

        const ANC_BUF_SIZE: usize = 1024;
        let mut anc_buf = [0 as c_char; ANC_BUF_SIZE];

        msg.msg_control = anc_buf.as_mut_ptr() as *mut c_void;
        msg.msg_controllen = ANC_BUF_SIZE;

        let recv_flags = libc::MSG_CMSG_CLOEXEC;
        if libc::recvmsg(socket, &mut msg as *mut libc::msghdr, recv_flags) == -1 {
            Err(io::Error::last_os_error())?;
        }

        if recv_buf[0] as u8 != HandleParcel::SANITY_BYTE {
            return Err(HandleParcelError::Protocol);
        }

        let pcmsg = libc::CMSG_FIRSTHDR(msg.msg_control as *const libc::msghdr);
        if pcmsg == ptr::null_mut() {
            return Err(HandleParcelError::Protocol);
        }
        let cmsg = &*pcmsg;
        if cmsg.cmsg_level != libc::SOL_SOCKET || cmsg.cmsg_type != libc::SCM_RIGHTS {
            return Err(HandleParcelError::Protocol);
        }
        if (cmsg.cmsg_len as u32) != libc::CMSG_LEN(mem::size_of::<Handle>() as u32) {
            return Err(HandleParcelError::Protocol);
        }
        let fd_ptr: *const Handle = libc::CMSG_DATA(cmsg) as *const Handle;

        let handle = *fd_ptr;

        Ok(HandleParcel(handle))
    }
}

impl Sock {
    /*    pub fn send<T: SimpleMessage>(&mut self, value: &T)
    where
        <T as FromStr>::Err: Send + Sync + Debug + error::Error,
    {
        let res = value.to_string();
        let buf = res.as_bytes();
        let buf_len = buf.len();
        assert!(buf_len <= 4096);
        unsafe {
            if libc::write(self.fd, buf.as_ptr() as *const _, buf_len) == -1 {
                err_exit("write");
            }
        }
    }

    pub fn receive<T: SimpleMessage>(&mut self) -> T
    where
        <T as FromStr>::Err: Send + Sync + Debug + error::Error,
    {
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
    }*/

    pub unsafe fn send<T: Message>(&self, x: &T) -> Result<(), T::Error> {
        T::send(x, self.fd)
    }

    pub unsafe fn receive<T: Message>(&self) -> Result<T, T::Error> {
        T::receive(self.fd)
    }

    pub unsafe fn make_pair() -> (Sock, Sock) {
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

impl FromStr for WaitMessage {
    type Err = <u16 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        Ok(Self { class: s.parse()? })
    }
}

impl<T> SimpleMessage for T
where
    T: 'static + Debug + Display + ToString + FromStr,
    <Self as FromStr>::Err: Send + Sync + Debug + error::Error,
{
}

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
