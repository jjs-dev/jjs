use crate::{config::Strategy, Options, Resolver, ToolchainSpec};
use anyhow::Context as _;
use serde::Deserialize;
use std::collections::HashSet;

pub(super) struct DebootstrapResolver {
    packages: HashSet<String>,
    options: Options,
}

#[derive(Deserialize)]
struct DebootstrapConfig {
    packages: Vec<String>,
}

impl DebootstrapResolver {
    pub(super) fn new(opt: &Options) -> DebootstrapResolver {
        DebootstrapResolver {
            packages: HashSet::new(),
            options: opt.clone(),
        }
    }
}

impl Resolver for DebootstrapResolver {
    fn strategy(&self) -> Strategy {
        Strategy::Debootstrap
    }

    fn strategy_name(&self) -> &'static str {
        "debootstrap"
    }

    fn visit_spec(
        &mut self,
        spec: &ToolchainSpec,
        _log: Option<&mut dyn std::io::Write>,
    ) -> anyhow::Result<()> {
        let config_file_path = spec.dir.join("debootstrap.yaml");
        let config = std::fs::read(&config_file_path).with_context(|| {
            format!("failed to read config from {}", config_file_path.display())
        })?;
        let config: DebootstrapConfig =
            serde_yaml::from_slice(&config).context("config parse error")?;
        for pkg in &config.packages {
            self.packages.insert(pkg.clone());
        }
        Ok(())
    }

    fn finish(&mut self) -> anyhow::Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }
        let sysroot_dir = self.options.out.join("opt");
        let mut cmd = std::process::Command::new("fakechroot");

        cmd.arg("fakeroot").arg("debootstrap");
        cmd.arg("--variant=minbase");
        cmd.arg("unstable");
        cmd.arg(&sysroot_dir);
        let status = cmd.status().context("failed to start debootstrap")?;
        if !status.success() {
            anyhow::bail!("debootstrap failed");
        }
        {
            let mut apt = std::process::Command::new("sudo");
            apt.arg("chroot");
            apt.arg(&sysroot_dir);
            apt.arg("apt");
            apt.arg("install");
            apt.arg("--no-install-recommends");
            apt.arg("--yes");
            for pkg in std::mem::take(&mut self.packages) {
                apt.arg(pkg);
            }
            let status = apt.status().context("failed to start apt")?;
            if !status.success() {
                anyhow::bail!("apt failed");
            }
        }
        Ok(())
    }
}
