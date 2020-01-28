use std::process::Command;
use util::cmd::CommandExt;

pub fn build_docker_image(
    params: &crate::Params,
    docker_cfg: &crate::cfg::DockerConfig,
    runner: &util::cmd::Runner,
) {
    println!("Building docker image");
    let mut cmd = Command::new("docker");
    let dockerfile_path = "./docker/Dockerfile";
    cmd.arg("build");
    for opt in &docker_cfg.build_options {
        cmd.arg(opt);
    }
    cmd.arg(&params.artifacts)
        .args(&["--file", &dockerfile_path]);
    let default_tag = "jjs";
    cmd.args(&["--tag", default_tag]);
    for tag in &docker_cfg.tag {
        cmd.args(&["--tag", tag]);
    }
    cmd.run_on(runner);
}
