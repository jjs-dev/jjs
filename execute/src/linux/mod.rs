mod dominion;
mod jail_common;
mod jobserver;
mod pipe;
mod util;

pub use crate::linux::dominion::{DesiredAccess, LinuxDominion};
use crate::{
    linux::{
        pipe::{LinuxReadPipe, LinuxWritePipe},
        util::{err_exit, Handle, IgnoreExt, Pid},
    },
    Backend, ChildProcess, ChildProcessOptions, DominionOptions, DominionPointerOwner, DominionRef,
    ErrorKind, HandleWrapper, InputSpecification, OutputSpecification, WaitOutcome,
};
use downcast_rs::Downcast;
use failure::ResultExt;
use nix::sys::memfd;
use std::{
    ffi::CString,
    fs,
    io::{Read, Write},
    os::unix::io::IntoRawFd,
    ptr,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex,
    },
    time::{self, Duration},
};

pub struct LinuxChildProcess {
    exit_code: AtomicI64,

    stdin: Option<Box<dyn Write>>,
    stdout: Option<Box<dyn Read>>,
    stderr: Option<Box<dyn Read>>,
    //in order to save dominion while CP is alive
    _dominion_ref: DominionRef,

    pid: Pid,
}

const EXIT_CODE_STILL_RUNNING: i64 = i64::min_value(); // It doesn't intersect with normal exit codes
                                                       // because they fit in i32
impl ChildProcess for LinuxChildProcess {
    fn get_exit_code(&self) -> crate::Result<Option<i64>> {
        self.poll()?;
        let ec = self.exit_code.load(Ordering::SeqCst);
        let ec = match ec {
            EXIT_CODE_STILL_RUNNING => None,
            w => Some(w),
        };
        Ok(ec)
    }

    fn get_stdio(
        &mut self,
    ) -> (
        Option<Box<dyn Write>>,
        Option<Box<dyn Read>>,
        Option<Box<dyn Read>>,
    ) {
        (self.stdin.take(), self.stdout.take(), self.stderr.take())
    }

    fn wait_for_exit(&self, timeout: std::time::Duration) -> crate::Result<WaitOutcome> {
        if self.exit_code.load(Ordering::SeqCst) != EXIT_CODE_STILL_RUNNING {
            return Ok(WaitOutcome::AlreadyFinished);
        }

        let mut logger = util::strace_logger();
        let mut d = self._dominion_ref.d.lock().unwrap();
        let d = (*d).b.downcast_mut::<LinuxDominion>().unwrap();

        write!(logger, "sending wait query");
        let wait_result = unsafe { d.poll_job(self.pid, timeout) };
        write!(logger, "wait returned: {:?}", wait_result);
        match wait_result {
            None => Ok(WaitOutcome::Timeout),
            Some(w) => {
                //self.exit_code = Some(AtomicI64::new(i64::from(w)));
                self.exit_code.store(i64::from(w), Ordering::SeqCst);
                Ok(WaitOutcome::Exited)
            }
        }
    }

    fn poll(&self) -> crate::Result<()> {
        self.wait_for_exit(Duration::from_nanos(1)).map(|_w| ())
    }

    fn is_finished(&self) -> crate::Result<bool> {
        self.poll()?;
        Ok(self.exit_code.load(Ordering::SeqCst) != EXIT_CODE_STILL_RUNNING)
    }

    fn kill(&mut self) -> crate::Result<()> {
        unsafe {
            if self.is_finished()? {
                return Ok(());
            }
            if libc::kill(self.pid, libc::SIGKILL) == -1 {
                err_exit("kill");
            }
            Ok(())
        }
    }
}

impl Drop for LinuxChildProcess {
    fn drop(&mut self) {
        let f = self.is_finished();
        if f.is_err() || f.unwrap() == false {
            return;
        }
        self.kill().ignore();
        self.wait_for_exit(time::Duration::from_millis(100))
            .unwrap();
    }
}

fn handle_input_io(spec: InputSpecification) -> crate::Result<(Option<Handle>, Handle)> {
    match spec {
        InputSpecification::Pipe => {
            let mut h_in = 0;
            let mut h_out = 0;
            pipe::setup_pipe(&mut h_in, &mut h_out)?;
            let f = unsafe { libc::dup(h_out) };
            unsafe { libc::close(h_in) };
            Ok((Some(h_out), h_in))
        }
        InputSpecification::RawHandle(HandleWrapper { h }) => {
            let h = h as Handle;
            Ok((None, h))
        }
        InputSpecification::Empty => {
            let file = fs::File::create("/dev/null").context(ErrorKind::Io)?;
            let file = file.into_raw_fd();
            Ok((None, file))
        }
        InputSpecification::Null => Ok((None, -1 as Handle)),
    }
}

fn handle_output_io(spec: OutputSpecification) -> crate::Result<(Option<Handle>, Handle)> {
    match spec {
        OutputSpecification::Null => Ok((None, -1 as Handle)),
        OutputSpecification::RawHandle(HandleWrapper { h }) => Ok((None, h as Handle)),
        OutputSpecification::Pipe => {
            let mut h_in = 0;
            let mut h_out = 0;
            pipe::setup_pipe(&mut h_in, &mut h_out)?;
            let f = unsafe { libc::dup(h_out) };
            unsafe { libc::close(h_out) };
            Ok((Some(h_in), f))
        }
        OutputSpecification::Ignore => {
            let file = fs::File::open("/dev/null").context(ErrorKind::Io)?;
            let file = file.into_raw_fd();
            let fd = unsafe { libc::dup(file) };
            Ok((None, fd))
        }
        OutputSpecification::Buffer(sz) => {
            let memfd_name = "libminion_output_memfd";
            let memfd_name = CString::new(memfd_name).unwrap();
            let mut flags = memfd::MemFdCreateFlag::MFD_CLOEXEC;
            if sz.is_some() {
                flags = flags | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING;
            }
            let mfd = memfd::memfd_create(&memfd_name, flags).unwrap();
            if let Some(sz) = sz {
                if unsafe { libc::ftruncate(mfd, sz as i64) } == -1 {
                    err_exit("ftruncate");
                }
            }
            let child_fd = unsafe { libc::dup(mfd) };
            Ok((Some(mfd), child_fd))
        }
    }
}

fn spawn(options: ChildProcessOptions) -> crate::Result<LinuxChildProcess> {
    unsafe {
        let mut logger = util::strace_logger();
        write!(logger, "linux:spawn() setting up requests");
        let q = jail_common::JobQuery {
            image_path: options.path.clone(),
            argv: options.arguments.clone(),
            environment: options
                .environment
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            pwd: options.pwd.clone(),
        };

        let (in_w, in_r) = handle_input_io(options.stdio.stdin)?;
        let (out_r, out_w) = handle_output_io(options.stdio.stdout)?;
        let (err_r, err_w) = handle_output_io(options.stdio.stderr)?;

        let q = dominion::ExtendedJobQuery {
            job_query: q,

            stdin: in_r,
            stdout: out_w,
            stderr: err_w,
        };
        let mut d = options.dominion.d.lock().unwrap();
        let d = d.b.downcast_mut::<LinuxDominion>().unwrap();

        write!(logger, "sending JobQuery to dominion");
        let ret = d.spawn_job(q);

        write!(logger, "creating pipes");
        let mut stdin = None;
        if let Some(h) = in_w {
            let box_in: Box<dyn Write> = Box::new(LinuxWritePipe::new(h));
            stdin.replace(box_in);
        }
        let mut stdout = None;
        if let Some(h) = out_r {
            let b: Box<dyn Read> = Box::new(LinuxReadPipe::new(h));
            stdout.replace(b);
        }
        let mut stderr = None;
        if let Some(h) = err_r {
            let b: Box<dyn Read> = Box::new(LinuxReadPipe::new(h));
            stderr.replace(b);
        }
        write!(logger, "done");
        Ok(LinuxChildProcess {
            exit_code: AtomicI64::new(EXIT_CODE_STILL_RUNNING),
            stdin,
            stdout,
            stderr,
            _dominion_ref: options.dominion.clone(),
            pid: ret.pid,
        })
    }
}

pub struct LinuxBackend {}

impl Backend for LinuxBackend {
    type ChildProcess = LinuxChildProcess;
    fn new_dominion(&self, options: DominionOptions) -> crate::Result<DominionRef> {
        let pd = Box::new(unsafe { LinuxDominion::create(options) });
        Ok(DominionRef {
            d: Arc::new(Mutex::new(DominionPointerOwner { b: pd })),
        })
    }

    fn spawn(&self, options: ChildProcessOptions) -> crate::Result<LinuxChildProcess> {
        spawn(options)
    }
}

fn empty_signal_handler(
    _signal_code: libc::c_int,
    _signal_info: *mut libc::siginfo_t,
    _ptr: *mut libc::c_void,
) {
}

fn fix_sigchild() {
    unsafe {
        let sa_ptr: *mut libc::sigaction = util::allocate_heap_variable();
        let mut sa = &mut *sa_ptr;
        sa.sa_sigaction = empty_signal_handler as *mut () as usize;
        libc::sigemptyset(&mut sa.sa_mask as *mut _);
        libc::sigaddset(&mut sa.sa_mask as *mut _, libc::SIGCHLD);
        sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
        if libc::sigaction(libc::SIGCHLD, sa_ptr, ptr::null_mut()) == -1 {
            err_exit("sigaction");
        }
    }
}

pub fn setup_execution_manager() -> LinuxBackend {
    fix_sigchild();
    LinuxBackend {}
}
