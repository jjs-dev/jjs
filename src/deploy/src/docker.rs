use std::process::Command;

pub fn build_docker_image(params: &crate::Params) {
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
        let st = cmd.status().expect("failed run docker");
        assert!(st.success());
    };
    process_component("frontend");
    process_component("invoker");
    process_component("tools");
}
