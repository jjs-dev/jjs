use std::process::Command;
use structopt::StructOpt;
use util::cmd::{CommandExt, Runner};

#[derive(StructOpt)]
pub(crate) struct TestArgs {
    #[structopt(long)]
    verbose: bool,
    #[structopt(long, short = "i")]
    integration_tests: bool,
    #[structopt(long)]
    pub(crate) fail_fast: bool,
    #[structopt(long)]
    skip_unit: bool,
}

fn run_integ_test(runner: &Runner) {
    println!("Compiling integration tests");
    if !Command::new("cargo")
        .current_dir("src/e2e")
        .arg("test")
        .arg("--no-run")
        .run_on(runner)
    {
        return;
    }

    println!("Running integration tests");
    // TODO: hacky. Probably it can be done better.
    let out = Command::new("cargo")
        .current_dir("src/e2e")
        .args(&["test"])
        .arg("--")
        .arg("--list")
        .output()
        .expect("failed list integration tests")
        .stdout;

    let out = String::from_utf8(out).expect("cargo output is not utf8");
    let mut cnt_tests = 0;
    let mut cnt_ok = 0;
    for line in out.lines() {
        if line.contains(": test") {
            let test_name = line
                .split_whitespace()
                .next()
                .expect("line is empty")
                .trim_end_matches(':');
            println!("Running: {}", test_name);
            let test_succ = Command::new("cargo")
                .current_dir("src/e2e")
                .args(&["test", test_name])
                .run_on(runner);
            cnt_tests += 1;
            if test_succ {
                cnt_ok += 1;
            }
        }
    }
    println!(
        "{} integration tests runned, {} successful",
        cnt_tests, cnt_ok
    );
}

fn run_unit_tests(args: &TestArgs, runner: &Runner) {
    let mut cmd = Command::new("cargo");
    cmd.args(&["test"]);
    cmd.arg("--workspace");
    cmd.args(&["--exclude", "all"]);
    if args.verbose {
        cmd.args(&["--", "--nocapture"]);
    }
    cmd.run_on(runner);
}

pub(crate) fn task_test(args: TestArgs, runner: &Runner) {
    if !args.skip_unit {
        run_unit_tests(&args, runner);
    }
    if args.integration_tests {
        run_integ_test(runner);
    }
}
