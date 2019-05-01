#![feature(never_type, nll)]

use cfg_if::cfg_if;
use minion;
use std::time::Duration;
use structopt::StructOpt;

static COMPILATION_TIME: &str = env!("MINION_CLI_COMPILATION_TIME");

static VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct EnvItem {
    name: String,
    value: String,
}

fn parse_env_item(src: &str) -> Result<EnvItem, !> {
    let p = src.find('=').unwrap();
    Ok(EnvItem {
        name: String::from(&src[0..p]),
        value: String::from(&src[p + 1..]),
    })
}

fn parse_path_exposition_item(src: &str) -> Result<minion::PathExpositionOptions, !> {
    let sep1 = match src.find(':') {
        Some(x) => x,
        None => panic!("--expose item must contain to colons(`:`), but no one was provided"),
    };
    let sep2 = match src[sep1 + 1..].find(':') {
        Some(x) => x + sep1 + 1,
        None => panic!("--expose item must contain two colone(`:`), but one was provided"),
    };
    let amask = &src[sep1 + 1..sep2];
    if amask.len() != 3 {
        panic!(
            "access mask must contain 3 chars (R, W, X flags), but {} provided",
            amask.len()
        );
    }
    let access = match amask {
        "rwx" => minion::DesiredAccess::Full,
        "r-x" => minion::DesiredAccess::Readonly,
        _ => panic!("unknown access mask {}. rwx or r-x expected", amask),
    };
    Ok(minion::PathExpositionOptions {
        src: (&src[..sep1]).to_string(),
        dest: (&src[sep2 + 1..]).to_string(),
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
    #[structopt(short = "e", long = "env", parse(try_from_str = "parse_env_item"))]
    env: Vec<EnvItem>,

    /// Max peak process count (including main)
    #[structopt(short = "n", long = "max-process-count", default_value = "16")]
    num_processes: usize,

    /// Max memory availible to isolated process
    #[structopt(short = "m", long = "memory-limit", default_value = "256000000")]
    memory_limit: usize,

    /// Total CPU time in milliseconds
    #[structopt(short = "t", long = "time-limit", default_value = "1000")]
    time_limit: u32,

    /// Print parsed argv
    #[structopt(long = "dump-argv")]
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
        parse(try_from_str = "parse_path_exposition_item")
    )]
    exposed_paths: Vec<minion::PathExpositionOptions>,

    /// Process working dir, relative to `isolation_root`
    #[structopt(short = "p", long = "pwd", default_value = "/")]
    pwd: String,
}

#[derive(StructOpt, Debug)]
#[structopt(version = "run `minion-cli version` for version details")]
enum Opt {
    /// Run subprocess
    #[structopt(name = "run")]
    Exec(ExecOpt),
    /// Print version and exit
    #[structopt(name = "version")]
    Version,
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
    setup_human_panic();
    let options: Opt = Opt::from_args();
    let options = match options {
        Opt::Version => {
            println!("Minion CLI v{}, compiled {}", VERSION, COMPILATION_TIME);
            return;
        }
        Opt::Exec(o) => o,
    };
    if options.dump_argv {
        println!("{:#?}", options);
    }
    let execution_manager = minion::setup();

    let dominion = execution_manager.new_dominion(minion::DominionOptions {
        max_alive_process_count: options.num_processes,
        memory_limit: options.memory_limit,
        isolation_root: options.isolation_root,
        exposed_paths: options.exposed_paths,
        time_limit: Duration::from_millis(u64::from(options.time_limit)),
    });

    let dominion = dominion.unwrap();

    let args = minion::ChildProcessOptions {
        path: options.executable,
        arguments: options.argv,
        environment: options
            .env
            .iter()
            .map(|v| (v.name.clone(), v.value.clone()))
            .collect(),
        dominion,
        stdio: minion::StdioSpecification {
            stdin: unsafe {
                minion::InputSpecification::RawHandle(
                    minion::HandleWrapper::new(0), /*our stdin handle*/
                )
            },
            stdout: unsafe {
                minion::OutputSpecification::RawHandle(
                    minion::HandleWrapper::new(1), /*our stdout handle*/
                )
            },
            stderr: unsafe {
                minion::OutputSpecification::RawHandle(
                    minion::HandleWrapper::new(2), /*our stderr handle*/
                )
            },
        },
        pwd: options.pwd.clone(),
    };
    if options.dump_minion_params {
        println!("{:#?}", args);
    }
    let cp = execution_manager.spawn(args).unwrap();
    let timeout = Duration::from_secs(3600);
    cp.wait_for_exit(timeout).unwrap();
    let exit_code = cp.get_exit_code().unwrap().unwrap();
    println!("---> Child process exited with code {} <---", exit_code);
}
