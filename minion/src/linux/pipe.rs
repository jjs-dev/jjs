use crate::linux::util::{err_exit, Handle};
use libc::c_void;
use std::io;

pub struct LinuxReadPipe {
    handle: Handle,
}

impl std::io::Read for LinuxReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::read(self.handle, buf.as_mut_ptr() as *mut c_void, buf.len());
            if ret == -1 {
                err_exit("read");
            }
            Ok(ret as usize)
        }
    }
}

impl LinuxReadPipe {
    pub(crate) fn new(handle: Handle) -> LinuxReadPipe {
        LinuxReadPipe { handle }
    }
}

impl Drop for LinuxReadPipe {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.handle);
        }
    }
}

pub struct LinuxWritePipe {
    handle: Handle,
}

impl Drop for LinuxWritePipe {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.handle);
        }
    }
}

impl LinuxWritePipe {
    pub(crate) fn new(handle: Handle) -> LinuxWritePipe {
        LinuxWritePipe { handle }
    }
}

impl io::Write for LinuxWritePipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::write(self.handle, buf.as_ptr() as *const c_void, buf.len());
            if ret == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(ret as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        unsafe {
            let ret = libc::fsync(self.handle);
            if ret == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }
}

pub(crate) fn setup_pipe(read_end: &mut Handle, write_end: &mut Handle) -> crate::Result<()> {
    unsafe {
        let mut ends = [0 as Handle; 2];
        let ret = libc::pipe2(ends.as_mut_ptr(), libc::O_CLOEXEC);
        if ret == -1 {
            err_exit("pipe");
        }
        *read_end = ends[0];
        *write_end = ends[1];
        Ok(())
    }
}
