use cfg_if::cfg_if;
use minion::{self, Dominion};
use std::time::Duration;
use structopt::StructOpt;

#[derive(Debug)]
struct EnvItem {
    name: String,
    value: String,
}

fn parse_env_item(src: &str) -> Result<EnvItem, &'static str> {
    let p = src.find('=').ok_or("Env item doesn't look like KEY=VAL")?;
    Ok(EnvItem {
        name: String::from(&src[0..p]),
        value: String::from(&src[p + 1..]),
    })
}

fn parse_path_exposition_item(src: &str) -> Result<minion::PathExpositionOptions, String> {
    let parts = src.splitn(3, ':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!(
            "--expose item must contain two colons (`:`), but no {} was provided",
            parts.len()
        ));
    }
    let amask = parts[1];
    if amask.len() != 3 {
        return Err(format!(
            "access mask must contain 3 chars (R, W, X flags), but {} provided",
            amask.len()
        ));
    }
    let access = match amask {
        "rwx" => minion::DesiredAccess::Full,
        "r-x" => minion::DesiredAccess::Readonly,
        _ => {
            return Err(format!(
                "unknown access mask {}. rwx or r-x expected",
                amask
            ));
        }
    };
    Ok(minion::PathExpositionOptions {
        src: parts[0].to_string().into(),
        dest: parts[2].to_string().into(),
        access,
    })
}

#[derive(StructOpt, Debug)]
struct ExecOpt {
    /// Full name of executable file (e.g. /bin/ls)
    #[structopt(name = "bin")]
    executable: String,

    /// Arguments for isolated process
    #[structopt(short = "a", long = "arg")]
    argv: Vec<String>,

    /// Environment variables (KEY=VAL) which will be passed to isolated process
    #[structopt(short = "e", long, parse(try_from_str = parse_env_item))]
    env: Vec<EnvItem>,

    /// Max peak process count (including main)
    #[structopt(short = "n", long = "max-process-count", default_value = "16")]
    num_processes: usize,

    /// Max memory available to isolated process
    #[structopt(short = "m", long, default_value = "256000000")]
    memory_limit: usize,

    /// Total CPU time in milliseconds
    #[structopt(short = "t", long, default_value = "1000")]
    time_limit: u32,

    /// Print parsed argv
    dump_argv: bool,

    /// Print libminion parameters
    #[structopt(long = "dump-generated-security-settings")]
    dump_minion_params: bool,

    /// Isolation root
    #[structopt(short = "r", long = "root")]
    isolation_root: String,

    /// Exposed paths (/source/path:MASK:/dest/path), MASK is r-x for readonly access and rwx for full access
    #[structopt(
        short = "x",
        long = "expose",
        parse(try_from_str = parse_path_exposition_item)
    )]
    exposed_paths: Vec<minion::PathExpositionOptions>,

    /// Process working dir, relative to `isolation_root`
    #[structopt(short = "p", long = "pwd", default_value = "/")]
    pwd: String,
}

cfg_if! {
if #[cfg(feature="human_panic")] {
    fn setup_human_panic() {
        human_panic::setup_panic!();
    }
} else {
    fn setup_human_panic() {

    }
}
}

fn main() {
    // TODO
    std::env::set_var("RUST_BACKTRACE", "1");
    setup_human_panic();
    let options: ExecOpt = ExecOpt::from_args();
    if options.dump_argv {
        println!("{:#?}", options);
    }
    let execution_manager = minion::setup();

    let dominion = execution_manager.new_dominion(minion::DominionOptions {
        max_alive_process_count: options.num_processes.min(u32::max_value() as usize) as u32,
        memory_limit: options.memory_limit as u64,
        isolation_root: options.isolation_root.into(),
        exposed_paths: options.exposed_paths,
        time_limit: Duration::from_millis(u64::from(options.time_limit)),
    });

    let dominion = dominion.unwrap();

    let (stdin_fd, stdout_fd, stderr_fd);
    unsafe {
        stdin_fd = libc::dup(0) as u64;
        stdout_fd = libc::dup(1) as u64;
        stderr_fd = libc::dup(2) as u64;
    }
    let args = minion::ChildProcessOptions {
        path: options.executable.into(),
        arguments: options.argv.iter().map(|x| x.into()).collect(),
        environment: options
            .env
            .iter()
            .map(|v| (v.name.clone().into(), v.value.clone().into()))
            .collect(),
        dominion: dominion.clone(),
        stdio: minion::StdioSpecification {
            stdin: unsafe { minion::InputSpecification::handle(stdin_fd) },
            stdout: unsafe { minion::OutputSpecification::handle(stdout_fd) },
            stderr: unsafe { minion::OutputSpecification::handle(stderr_fd) },
        },
        pwd: options.pwd.into(),
    };
    if options.dump_minion_params {
        println!("{:#?}", args);
    }
    let cp = execution_manager.spawn(args).unwrap();
    let timeout = Duration::from_secs(3600);
    cp.wait_for_exit(timeout).unwrap();
    let exit_code = cp.get_exit_code().unwrap();
    println!("---> Child process exited with code {:?} <---", exit_code);
    if dominion.check_cpu_tle().unwrap() {
        println!("Note: CPU time limit was exceeded");
    }
    if dominion.check_real_tle().unwrap() {
        println!("Note: wall-clock time limit was exceeded");
    }
}
