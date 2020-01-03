//! Generates Debian packages (.deb)

use crate::{print_section, Params};

use std::process::Command;
use util::cmd::CommandExt;

pub fn create(params: &Params, runner: &util::cmd::Runner) {
    print_section("Creating package");
    let mut cmd = Command::new("bash");

    cmd.current_dir("deb").arg("build.sh");

    let build_dir = params.build.join("deb-build");
    std::fs::remove_dir_all(&build_dir).ok();
    cmd.arg("--build-dir").arg(build_dir);

    let archive_path = params.build.join("jjs.tgz");
    cmd.arg("--archive-path").arg(archive_path);

    let out_path = params.artifacts.join("pkg/jjs.deb");
    cmd.arg("--out").arg(out_path);
    cmd.run_on(runner);
}
