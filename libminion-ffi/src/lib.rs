use execute;
use std::{time, os::raw::c_char, ffi::{CString, CStr}};

pub struct Backend;

#[no_mangle]
pub unsafe extern "C" fn minion_setup() -> *mut Backend {
    let backend = execute::setup();
    let backend = Box::into_raw(backend);
    backend as *mut Backend
}

pub struct DominionOptionsWrapper {
    opts: execute::DominionOptions,
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_options_create() -> *mut DominionOptionsWrapper {
    let opts = DominionOptionsWrapper {
        opts: execute::DominionOptions {
            allow_network: false,
            allow_file_io: false,
            max_alive_process_count: 0,
            memory_limit: 0,
            time_limit: time::Duration::new(0, 0),
            isolation_root: "".into(),
            exposed_paths: vec![]
        }
    };
    let opts = Box::new(opts);
    let opts = Box::into_raw(opts);
    opts
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_options_time_limit(options: *mut DominionOptionsWrapper, seconds: u32, nanos: u32) {
    (*options).opts.time_limit = time::Duration::new( u64::from(seconds), nanos)
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_options_process_limit(options: *mut DominionOptionsWrapper, new_val: u32) {
    (*options).opts.max_alive_process_count = new_val as usize
}

#[no_mangle]
pub unsafe extern "C" fn minion_dominion_options_isolation_root(options: *mut DominionOptionsWrapper, path: *const c_char) {
    let str = CStr::from_ptr(path).to_str().unwrap().to_string();
    (*options).opts.isolation_root = str;
}

#[no_ma]