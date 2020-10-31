use anyhow::Context;
use clap::Clap;
use std::process::Command;
use util::cmd::{CommandExt, Runner};

#[derive(Clap)]
pub(crate) struct TestArgs {
    #[clap(long)]
    verbose: bool,
    #[clap(long, short = 'i')]
    integration_tests: bool,
    #[clap(long)]
    pub(crate) fail_fast: bool,
    #[clap(long)]
    skip_unit: bool,
    /// Nocapture e2e
    #[clap(long)]
    nocapture: bool,
}

fn run_integ_test(runner: &Runner, nocapture: bool) -> anyhow::Result<()> {
    println!("Compiling integration tests");
    Command::new("cargo")
        .current_dir("src/e2e")
        .arg("test")
        .arg("--no-run")
        .try_exec()
        .context("failed to compile tests")?;

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
            println!("----- Running: {} -----", test_name);
            let mut cmd = Command::new("cargo");
            cmd.current_dir("src/e2e")
                .args(&["test", test_name])
                .arg("--")
                .arg("-Zunstable-options")
                .arg("--ensure-time")
                .arg("--report-time")
                .env("RUST_TEST_TIME_INTEGRATION", "45000,120000");
            if nocapture {
                cmd.arg("--nocapture");
            }
            let test_succ = cmd.try_exec().is_ok();
            cnt_tests += 1;
            if test_succ {
                cnt_ok += 1;
            } else {
                runner.error();
            }
        }
    }
    println!("{} integration tests ran, {} successful", cnt_tests, cnt_ok);
    Ok(())
}

fn run_unit_tests(args: &TestArgs, runner: &Runner) {
    let mut cmd = Command::new("cargo");
    cmd.args(&["test"]);
    cmd.arg("--workspace");
    cmd.args(&["--exclude", "e2e"]);
    if args.verbose {
        cmd.args(&["--", "--nocapture"]);
    }
    cmd.run_on(runner);
}

pub(crate) fn task_test(args: TestArgs, runner: &Runner) -> anyhow::Result<()> {
    if !args.skip_unit {
        run_unit_tests(&args, runner);
    }
    if args.integration_tests {
        run_integ_test(runner, args.nocapture)?;
    }
    Ok(())
}
