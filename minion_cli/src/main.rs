#![feature(never_type, nll)]

use cfg_if::cfg_if;
use execute::{self, Backend, ChildProcess};
use std::io::{self, copy, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;
use structopt::StructOpt;

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

fn parse_path_exposition_item(src: &str) -> Result<execute::PathExpositionOptions, !> {
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
    let amask: Vec<_> = amask.chars().collect();
    let (r, w, x);
    let flag_parser = |pos, flag_name, true_val| match amask[pos] {
        y if y == true_val => true,
        '-' => false,
        _ => panic!("{} flag must be either '{}' or '-'", flag_name, true_val),
    };
    r = flag_parser(0, 'R', 'r');
    w = flag_parser(1, 'W', 'w');
    x = flag_parser(2, 'X', 'x');
    Ok(execute::PathExpositionOptions {
        src: (&src[..sep1]).to_string(),
        dest: (&src[sep2 + 1..]).to_string(),
        allow_read: r,
        allow_write: w,
        allow_execute: x,
    })
}

#[derive(StructOpt, Debug)]
struct Opt {
    ///full name of executable file (e.g. /bin/ls)
    #[structopt(name = "bin")]
    executable: String,

    ///unused
    #[structopt(short = "i", long = "isolation")]
    isolation: bool,

    ///arguments for isolated process
    #[structopt(short = "a", long = "arg")]
    argv: Vec<String>,

    ///environment variables (KEY=VAL) which will be passed to isolated process
    #[structopt(short = "e", long = "env", parse(try_from_str = "parse_env_item"))]
    env: Vec<EnvItem>,

    ///max peak process count (including main)
    #[structopt(short = "n", long = "max-process-count", default_value = "16")]
    num_processes: usize,

    ///max memory availible to isolated process
    #[structopt(short = "m", long = "memory-limit", default_value = "256000000")]
    memory_limit: usize,
    ///print parsed argv
    #[structopt(short = "r", long = "dump-argv")]
    dump_argv: bool,

    ///print libminion parameters
    #[structopt(short = "d", long = "dump-generated-security-settings")]
    dump_minion_params: bool,

    ///isolation root
    #[structopt(short = "p", long = "isolation-root")]
    isolation_root: String,

    ///exposed paths (/source/path:MASK:/dest/path), MASK is ignored, possible value is ---
    #[structopt(
    short = "x",
    long = "expose",
    parse(try_from_str = "parse_path_exposition_item")
    )]
    exposed_paths: Vec<execute::PathExpositionOptions>,
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
    if options.dump_argv {
        println!("{:#?}", options);
    }
    let mut execution_manager = execute::setup();

    let dominion = execution_manager.new_dominion(execute::DominionOptions {
        allow_network: false,
        allow_file_io: false,
        max_alive_process_count: options.num_processes,
        memory_limit: options.memory_limit,
        isolation_root: options.isolation_root.into(),
        exposed_paths: options.exposed_paths,
    });

    let dominion = dominion;

    let args = execute::ChildProcessOptions {
        path: options.executable,
        arguments: options.argv,
        environment: options
            .env
            .iter()
            .map(|v| (v.name.clone(), v.value.clone()))
            .collect(),
        dominion,
    };
    if options.dump_minion_params {
        println!("{:#?}", args);
    }
    let mut cp = execution_manager.spawn(args);

    let stdio = cp.get_stdio().unwrap();

    let (mut stdin, mut stdout, mut stderr) = stdio.split();

    let mut p = scoped_threadpool::Pool::new(4);
    let is_exited = AtomicBool::new(false);
    p.scoped(|scope| {
        scope.execute(|| {
            copy(&mut stdout, &mut io::stdout()).unwrap();
        });
        scope.execute(|| {
            copy(&mut stderr, &mut io::stderr()).unwrap();
        });
        scope.execute(|| {
            let mut buf = [0; 1024];
            while !is_exited.load(Ordering::SeqCst) {
                let res = io::stdin().read(&mut buf);
                sleep(Duration::from_millis(50));
                let res = match res {
                    Ok(x) => x,
                    x => x.unwrap(),
                };

                match stdin.write(&buf[..res]) {
                    Ok(_) => (),
                    Err(e) => match e.kind() {
                        io::ErrorKind::BrokenPipe => (),
                        _ => Err(e).unwrap(),
                    },
                }
            }
        });
        scope.execute(|| {
            let timeout = Duration::from_secs(3600);
            cp.wait_for_exit(timeout).unwrap();
            let exit_code = cp.get_exit_code().unwrap();
            println!("---child process exited with code {}---", exit_code);
            is_exited.store(true, Ordering::SeqCst);
        })
    })
}
