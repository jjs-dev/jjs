use std::process::Command;
use util::cmd::CommandExt;

pub fn build_docker_image(params: &crate::Params, runner: &util::cmd::Runner) {
    let process_component = |name: &str| {
        println!("Building docker image for {}", name);
        let mut cmd = Command::new("docker");
        let dockerfile_path = format!("./docker/{}.Dockerfile", name);
        cmd.arg("build")
            .arg(&params.artifacts)
            .args(&["--file", &dockerfile_path]);
        if let Some(tag) = &params.cfg.docker_tag {
            let tag = tag.replace('%', name);
            cmd.args(&["--tag", &tag]);
        }
        cmd.run_on(runner);
    };
    process_component("frontend");
    process_component("invoker");
    process_component("tools");
}
