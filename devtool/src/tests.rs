use crate::{CommandExt, Runner};
use std::process::Command;
use structopt::StructOpt;

#[derive(StructOpt)]
pub(crate) struct TestArgs {
    #[structopt(long)]
    verbose: bool,
    #[structopt(long, short = "i")]
    integration_tests: bool,
}

fn run_integ_test(runner: &Runner) {
    println!("Running integration tests");
    // TODO: hacky. Probably it can be done better.
    let out = Command::new("cargo")
        .current_dir("all")
        .args(&["test"])
        .arg("--")
        .arg("--list")
        .output()
        .expect("failed list integration tests")
        .stdout;

    let out = String::from_utf8(out).expect("cargo output is not utf8");
    for line in out.lines() {
        if line.contains(": test") {
            let test_name = line
                .split_whitespace()
                .next()
                .expect("line is empty")
                .trim_end_matches(":");
            println!("Running: {}", test_name);
            Command::new("cargo")
                .current_dir("all")
                .args(&["test", test_name])
                .run_on(runner);
        }
    }
}

pub(crate) fn task_test(args: TestArgs, runner: &Runner) {
    let mut cmd = Command::new("cargo");
    cmd.args(&["test"]);
    cmd.arg("--workspace");
    cmd.args(&["--exclude", "all"]);

    if args.verbose {
        cmd.args(&["--", "--nocapture"]);
    }
    if args.integration_tests {
        run_integ_test(runner);
    }
    cmd.run_on(runner);
}
