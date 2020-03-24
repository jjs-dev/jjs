use std::ffi::CString;

pub fn buffer_to_file(buf: &[u8], comment: &str) -> i64 {
    use nix::{
        fcntl::{self, FcntlArg},
        sys::memfd::{self, MemFdCreateFlag},
    };
    let fd = memfd::memfd_create(
        &CString::new(comment).unwrap(),
        MemFdCreateFlag::MFD_ALLOW_SEALING,
    )
    .unwrap();
    let mut buf_rem = buf;
    loop {
        let cnt = nix::unistd::write(fd, buf_rem).unwrap();
        buf_rem = &buf_rem[cnt..];
        if cnt == 0 {
            break;
        }
    }
    // now seal memfd
    // currently this is not important, but when...
    // TODO: cache all this stuff
    // ... it is important that file can't be altered by solution
    let seals = libc::F_SEAL_GROW | libc::F_SEAL_SEAL | libc::F_SEAL_WRITE | libc::F_SEAL_SHRINK;
    fcntl::fcntl(
        fd,
        FcntlArg::F_ADD_SEALS(fcntl::SealFlag::from_bits(seals).unwrap()),
    )
    .unwrap();
    // and seek fd to begin
    nix::unistd::lseek64(fd, 0, nix::unistd::Whence::SeekSet).unwrap();
    i64::from(fd)
}

// this is bug in clippy: https://github.com/rust-lang/rust-clippy/issues/5368
// #[allow(clippy::verbose_file_reads)]
pub fn handle_read_all(h: i64) -> Vec<u8> {
    use std::{io::Read, os::unix::io::FromRawFd};
    let h = h as i32;
    let mut file = unsafe { std::fs::File::from_raw_fd(h) };
    let mut out = Vec::new();
    let res = file.read_to_end(&mut out);
    res.unwrap();
    out
}

pub fn make_pipe() -> (i64, i64) {
    let (a, b) = nix::unistd::pipe().unwrap();
    (i64::from(a), i64::from(b))
}

pub fn handle_inherit(h: i64, close: bool) -> i64 {
    let out = i64::from(nix::unistd::dup(h as i32).unwrap());
    if close {
        nix::unistd::close(h as i32).unwrap()
    }

    out
}

pub fn close(h: i64) {
    nix::unistd::close(h as i32).unwrap()
}
