#![feature(try_trait)]
#![allow(unused_attributes)] //FIXME: workaround for https://github.com/rust-lang/rust/issues/60050

use minion;
use std::{collections::HashMap, ffi::CStr, mem, os::raw::c_char};

#[repr(i32)]
pub enum ErrorCode {
    /// operation completed successfully
    Ok,
    /// passed arguments didn't pass some basic checks
    /// examples:
    /// - provided buffer was expected to be null-terminated utf8-encoded string, but wasn't
    /// - something was expected to be unique, but wasn't, and so on
    /// these errors usually imply bug exists in caller code
    InvalidInput,
    /// unknown error
    Unknown,
}

pub enum GetStringResult {
    String(String),
    Error,
}

unsafe fn get_string(buf: *const c_char) -> GetStringResult {
    let r = CStr::from_ptr(buf)
        .to_str()
        .map(|s| GetStringResult::String(s.to_string()))
        .map_err(|_| GetStringResult::Error);
    match r {
        Ok(r) => r,
        Err(r) => r,
    }
}

impl std::ops::Try for ErrorCode {
    type Ok = ErrorCode;
    type Error = ErrorCode;

    fn into_result(self) -> Result<ErrorCode, ErrorCode> {
        match self {
            ErrorCode::Ok => Ok(ErrorCode::Ok),
            oth => Err(oth),
        }
    }

    fn from_ok(x: ErrorCode) -> Self {
        x
    }

    fn from_error(x: ErrorCode) -> Self {
        x
    }
}

impl std::ops::Try for GetStringResult {
    type Ok = String;
    type Error = ErrorCode;

    fn into_result(self) -> Result<String, ErrorCode> {
        match self {
            GetStringResult::String(x) => Ok(x),
            GetStringResult::Error => Err(ErrorCode::InvalidInput),
        }
    }

    fn from_error(_: ErrorCode) -> Self {
        unimplemented!()
    }

    fn from_ok(_: String) -> Self {
        unimplemented!()
    }
}

pub struct Backend(Box<dyn minion::Backend>);

/// Must be called before any library usage
#[no_mangle]
pub unsafe extern "C" fn minion_lib_init() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[minion-ffi] PANIC: {:?}", info);
        std::process::abort();
    }));
}

#[no_mangle]
pub unsafe extern "C" fn minion_backend_create(out: *mut *mut Backend) {
    let backend = Backend(minion::setup());
    let backend = Box::new(backend);
    *out = Box::into_raw(backend);
}

#[no_mangle]
pub unsafe extern "C" fn minion_backend_free(b: *mut Backend) {
    let b = Box::from_raw(b);
    mem::drop(b);
}

#[repr(C)]
pub struct TimeSpec {
    pub seconds: u32,
    pub nanoseconds: u32,
}

#[repr(C)]
pub struct DominionOptions {
    pub time_limit: TimeSpec,
    pub process_limit: u32,
    pub memory_limit: u32,
    pub isolation_root: *const c_char,
    pub shared_directories: *mut SharedDirectoryAccess,
}

#[derive(Clone)]
pub struct DominionWrapper(minion::DominionRef);

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_create(
    backend: *mut Backend,
    options: DominionOptions,
    out: *mut *mut DominionWrapper,
) -> ErrorCode {
    let mut exposed_paths = Vec::new();
    {
        let mut p = options.shared_directories;
        while !(*p).host_path.is_null() {
            let opt = minion::PathExpositionOptions {
                src: get_string((*p).host_path)?,
                dest: get_string((*p).sandbox_path)?,
                access: match (*p).kind {
                    SharedDirectoryAccessKind::Full => minion::DesiredAccess::Full,
                    SharedDirectoryAccessKind::ReadOnly => minion::DesiredAccess::Readonly,
                },
            };
            exposed_paths.push(opt);
            p = p.offset(1);
        }
    }
    let opts = minion::DominionOptions {
        max_alive_process_count: options.process_limit as _,
        memory_limit: options.memory_limit as _,
        time_limit: std::time::Duration::new(
            options.time_limit.seconds.into(),
            options.time_limit.nanoseconds,
        ),
        isolation_root: get_string(options.isolation_root)?,
        exposed_paths,
    };
    let backend = &(*backend);
    let d = backend.0.new_dominion(opts);
    let d = d.unwrap();

    let dw = DominionWrapper(d);
    *out = Box::into_raw(Box::new(dw));
    ErrorCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_clone(
    d_in: *mut DominionWrapper,
    out1: *mut *mut DominionWrapper,
    out2: *mut *mut DominionWrapper,
) {
    let dw = Box::from_raw(d_in);
    let dw = (*dw).0;
    let dref1 = dw.clone();
    let dref2 = dw;
    *out1 = Box::into_raw(Box::new(DominionWrapper(dref1)));
    *out2 = Box::into_raw(Box::new(DominionWrapper(dref2)));
}

#[repr(C)]
pub struct EnvItem {
    pub name: *const c_char,
    pub value: *const c_char,
}
#[repr(C)]
pub enum StdioMember {
    Stdin,
    Stdout,
    Stderr,
}

#[repr(C)]
pub struct StdioHandleSet {
    pub stdin: u64,
    pub stdout: u64,
    pub stderr: u64,
}
#[repr(C)]
pub struct ChildProcessOptions {
    pub image_path: *const c_char,
    pub argv: *const *const c_char,
    pub envp: *const EnvItem,
    pub stdio: StdioHandleSet,
    pub dominion: *mut DominionWrapper,
    pub workdir: *const c_char,
}

#[repr(C)]
pub enum SharedDirectoryAccessKind {
    Full,
    ReadOnly,
}

#[repr(C)]
pub struct SharedDirectoryAccess {
    pub kind: SharedDirectoryAccessKind,
    pub host_path: *const c_char,
    pub sandbox_path: *const c_char,
}

pub struct ChildProcessWrapper(Box<dyn minion::ChildProcess>);

#[no_mangle]
pub unsafe extern "C" fn minion_cp_spawn(
    backend: *mut Backend,
    options: ChildProcessOptions,
    out: *mut *mut ChildProcessWrapper,
) -> ErrorCode {
    let mut arguments = Vec::new();
    {
        let p = options.argv;
        while !(*p).is_null() {
            arguments.push(get_string(*p)?);
        }
    }
    let mut environment = HashMap::new();
    {
        let p = options.envp;
        while !(*p).name.is_null() {
            let name = get_string((*p).name)?;
            let value = get_string((*p).value)?;
            if environment.contains_key(&name) {
                return ErrorCode::InvalidInput;
            }
            environment.insert(name, value);
        }
    }
    let stdio = minion::StdioSpecification {
        stdin: minion::InputSpecification::RawHandle(minion::HandleWrapper::new(
            options.stdio.stdin,
        )),
        stdout: minion::OutputSpecification::RawHandle(minion::HandleWrapper::new(
            options.stdio.stdout,
        )),
        stderr: minion::OutputSpecification::RawHandle(minion::HandleWrapper::new(
            options.stdio.stderr,
        )),
    };
    let options = minion::ChildProcessOptions {
        path: get_string(options.image_path)?,
        arguments,
        environment,
        dominion: (*options.dominion).0.clone(),
        stdio,
        pwd: get_string(options.workdir)?,
    };
    let cp = (*backend).0.spawn(options).unwrap();
    let cp = ChildProcessWrapper(cp);
    let cp = Box::new(cp);
    *out = Box::into_raw(cp);
    ErrorCode::Ok
}
