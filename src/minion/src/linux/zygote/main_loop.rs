use crate::linux::{
    jail_common::{JobQuery, Query},
    util::{Handle, IpcSocketExt, Pid, StraceLogger},
    zygote::{setup, spawn_job, JobOptions, SetupData, Stdio, ZygoteOptions},
};
use std::{
    ffi::{OsStr, OsString},
    io::Write,
    os::unix::ffi::{OsStrExt, OsStringExt},
    time::Duration,
};

fn concat_env_item(k: &OsStr, v: &OsStr) -> OsString {
    let k = k.as_bytes();
    let v = v.as_bytes();
    let cap = k.len() + 1 + v.len();

    let mut res = vec![0; cap];
    res[0..k.len()].copy_from_slice(k);
    res[k.len() + 1..].copy_from_slice(v);
    res[k.len()] = b'=';
    OsString::from_vec(res)
}

unsafe fn process_spawn_query(
    arg: &mut ZygoteOptions,
    options: &JobQuery,
    setup_data: &SetupData,
) -> crate::Result<()> {
    let mut logger = StraceLogger::new();
    write!(logger, "got Spawn request").ok();
    //now we do some preprocessing
    let env: Vec<_> = options
        .environment
        .iter()
        .map(|(k, v)| concat_env_item(OsStr::from_bytes(&base64::decode(k).unwrap()), v))
        .collect();

    let mut child_fds = arg
        .sock
        .recv_struct::<u64, [Handle; 3]>()
        .unwrap()
        .1
        .unwrap();
    for f in child_fds.iter_mut() {
        *f = nix::unistd::dup(*f).unwrap();
    }
    let child_stdio = Stdio::from_fd_array(child_fds);

    let job_options = JobOptions {
        exe: options.image_path.clone(),
        argv: options.argv.clone(),
        env,
        stdio: child_stdio,
        pwd: options.pwd.clone().into_os_string(),
    };

    write!(logger, "JobOptions are fetched").ok();
    let startup_info = spawn_job(job_options, setup_data)?;
    write!(logger, "job started. Sending startup_info back").ok();
    arg.sock.send(&startup_info)?;
    Ok(())
}

unsafe fn process_poll_query(
    arg: &mut ZygoteOptions,
    pid: Pid,
    timeout: Duration,
) -> crate::Result<()> {
    let res = super::timed_wait(pid, timeout)?;
    arg.sock.send(&res)?;
    Ok(())
}

pub(crate) unsafe fn zygote_entry(mut arg: ZygoteOptions) -> crate::Result<i32> {
    let setup_data = setup::setup(&arg.jail_options, &mut arg.sock)?;

    let mut logger = StraceLogger::new();
    loop {
        let query: Query = match arg.sock.recv() {
            Ok(q) => {
                write!(logger, "zygote: new request").ok();
                q
            }
            Err(err) => {
                write!(logger, "zygote: got unprocessable query: {}", err).ok();
                return Ok(23);
            }
        };
        match query {
            Query::Spawn(ref o) => process_spawn_query(&mut arg, o, &setup_data)?,
            Query::Exit => break,
            Query::Poll(p) => process_poll_query(&mut arg, p.pid, p.timeout)?,
        };
    }
    Ok(0)
}
