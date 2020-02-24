use super::{Entity, Seal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Toolchain {
    /// Human-readable
    pub title: String,

    /// Machine-readable
    pub name: String,

    pub filename: String,

    #[serde(rename = "build")]
    pub build_commands: Vec<Command>,

    #[serde(rename = "run")]
    pub run_command: Command,

    #[serde(rename = "build-limits", default)]
    pub limits: pom::Limits,

    #[serde(rename = "env", default)]
    pub env: HashMap<String, String>,

    #[serde(rename = "env-passing", default)]
    pub env_passing: bool,

    #[serde(rename = "env-blacklist", default)]
    pub env_blacklist: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Command {
    #[serde(default = "Command::default_env")]
    pub env: HashMap<String, String>,
    pub argv: Vec<String>,
    #[serde(default = "Command::default_cwd")]
    pub cwd: String,
}

impl Command {
    fn default_env() -> HashMap<String, String> {
        HashMap::new()
    }

    fn default_cwd() -> String {
        String::from("/jjs")
    }
}
fn command_inherit_env(cmd: &mut Command, dfl: &HashMap<String, String>) {
    for (key, val) in dfl.iter() {
        cmd.env.entry(key.clone()).or_insert_with(|| val.clone());
    }
}
impl Seal for Toolchain {}
impl Entity for Toolchain {
    fn name(&self) -> &str {
        &self.name
    }

    fn postprocess(&mut self) -> anyhow::Result<()> {
        let mut inherit_env = self.env.clone();
        if self.env_passing {
            for (key, value) in std::env::vars() {
                if self.env_blacklist.contains(&key) {
                    continue;
                }
                inherit_env.entry(key).or_insert(value);
            }
        }

        for mut cmd in &mut self.build_commands {
            command_inherit_env(&mut cmd, &inherit_env);
        }
        command_inherit_env(&mut self.run_command, &inherit_env);

        Ok(())
    }
}
