use crate::{artifact::Artifact, cfg::DockerConfig, package::OtherPackage, Params};
use anyhow::Context as _;
use std::{io::Write, path::Path, process::Command};
use util::cmd::CommandExt;

pub(crate) struct DockerEmitter;

impl DockerEmitter {
    fn emit_inner(
        params: &Params,
        docker_context: &Path,
        pkg_name: &str,
        options: &DockerConfig,
    ) -> anyhow::Result<()> {
        let mut cmd = Command::new(&params.cfg.build.tool_info.docker);
        cmd.arg("build");
        cmd.arg("-f");
        cmd.arg(params.src.join("src").join(pkg_name).join("Dockerfile"));
        let tag = options
            .tag
            .clone()
            .unwrap_or_else(|| "jjs-%:latest".to_string())
            .replace('%', pkg_name);
        cmd.arg("-t").arg(&tag);
        cmd.arg(docker_context);
        cmd.try_exec()
            .with_context(|| format!("Failed to build image for package {}", pkg_name))?;
        if let Some(tag_log) = &options.write_tags_to_file {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(tag_log)
                .context("docker tag log unaccessible")?;
            writeln!(file, "{}", tag)?;
        }
        Ok(())
    }

    pub(crate) fn emit(
        &self,
        artifacts: &Vec<Artifact>,
        other_packages: &Vec<OtherPackage>,
        params: &Params,
        options: &DockerConfig,
    ) -> anyhow::Result<()> {
        // in fact, we just want to build each dockerfile using $BUILD/jjs-out
        // as context. Unfortunately, docker will copy this dir for every image
        // which is quite expensive. To reduce time complexity from O(N*S*S) to
        // O(N*S), we create separate context dir for each image
        crate::fs_util::ensure_exists(params.build.join("dockers"))?;
        println!("Building docker images");
        for artifact in artifacts {
            let ctx_dir = params.build.join("dockers").join(&artifact.package_name);
            std::fs::create_dir_all(&ctx_dir).context("mkdir failed")?;

            std::fs::copy(
                params.build.join("jjs-out").join(&artifact.package_name),
                ctx_dir.join(&artifact.package_name),
            )
            .context("context preparation error")?;
            Self::emit_inner(params, &ctx_dir, &artifact.package_name, &options)?;
        }
        for oth_pkg in other_packages {
            Self::emit_inner(
                params,
                &params.src.join("src").join(&oth_pkg.name),
                &oth_pkg.name,
                &options,
            )?;
        }
        Ok(())
    }
}
