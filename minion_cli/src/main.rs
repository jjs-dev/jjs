#![feature(never_type, nll, dbg_macro)]

use execute::{ChildProcess, ExecutionManager, self};
use std::io::{stdin, Read, Write};
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
        panic!("access mask must contain 3 chars (R, W, X flags), but {} provided", amask.len());
    }
    let amask: Vec<_> = amask.chars().collect();
    let (r, w, x);
    let flag_parser = |pos, flag_name, true_val| {
        match amask[pos] {
            y if y == true_val => true,
            '-' => false,
            _ => panic!("{} flag must be either '{}' or '-'", flag_name, true_val),
        }
    };
    r = flag_parser(0, 'R', 'r');
    w = flag_parser(1, 'W', 'w');
    x = flag_parser(2, 'X', 'x');
    Ok(
        execute::PathExpositionOptions {
            src: (&src[..sep1]).to_string(),
            dest: (&src[sep2 + 1..]).to_string(),
            allow_read: r,
            allow_write: w,
            allow_execute: x,
        }
    )
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

    ///exposed paths (/source/path:MASK:/dest/path), MASK can be e.g. r-x (deny writes)
    #[structopt(short = "x", long = "expose", parse(try_from_str = "parse_path_exposition_item"))]
    exposed_paths: Vec<execute::PathExpositionOptions>,
}

fn print_read_stream<R: Read>(mut r: R) {
    loop {
        let mut buf = [0 as u8; 1024];
        let res = r.read(&mut buf).unwrap();
        if res == 0 {
            break;
        }
        let s = String::from_utf8_lossy(&buf[..res]).to_string();
        print!("{}", s);
    }
}

fn type_write_stream<W: Write>(mut w: W) {
    loop {
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
        w.write(buf.as_bytes()).unwrap();
    }
}

fn main() {
    if cfg!(target_os = "linux") {
        let user_name = whoami::username();
        if user_name != "root" {
            eprintln!(
                "minion_cli is launched from '{}'\nOnly root is supported",
                user_name
            );
            std::process::exit(1);
        }
    }
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

    let (stdin, stdout, stderr) = stdio.split();

    let mut p = scoped_threadpool::Pool::new(3);
    p.scoped(|scope| {
        scope.execute(|| {
            print_read_stream(stdout);
        });
        scope.execute(|| {
            print_read_stream(stderr);
        });
        scope.execute(|| type_write_stream(stdin));
        scope.execute(|| {
            let timeout = std::time::Duration::from_secs(3600);
            cp.wait_for_exit(timeout).unwrap();
            //let exit_code= cp.get_exit_code().unwrap();
            println!("---child process exited---");
            std::process::exit(0);
        })
    })
}
