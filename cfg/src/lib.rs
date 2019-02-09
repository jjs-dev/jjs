extern crate serde;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use std::{collections::HashMap, env, fs, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Command {
    #[serde(default = "Command::default_env")]
    pub env: HashMap<String, String>,
    pub argv: Vec<String>,
    #[serde(default = "Command::default_cwd")]
    pub cwd: PathBuf,
}

impl Command {
    fn default_env() -> HashMap<String, String> {
        HashMap::new()
    }

    fn default_cwd() -> PathBuf {
        PathBuf::from(".")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Toolchain {
    pub name: String,
    pub suffix: String,
    pub build_commands: Vec<Command>,
    pub run_command: Command,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub toolchains: Vec<Toolchain>,
    #[serde(skip)]
    pub sysroot: String,
}

pub fn parse_file(path: PathBuf) -> Config {
    let file_content = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "Couldn't read main config file {:?} due to error {:?}",
            &path, e
        )
    });
    let raw_data: toml::Value = file_content.parse().unwrap();
    match toml::from_str(&file_content) {
        Ok(x) => x,
        Err(e) => panic!(
            "Error ocured when parsing config: {:?}.\nRaw config:\n{:#?}",
            e, raw_data
        ),
    }
}

pub fn get_config() -> Config {
    let sysroot = env::var("JJS_SYSROOT").expect("Sysroot must be provided in JJS_SYSROOT");
    let mut c = parse_file(PathBuf::from(format!("{}/etc/jjs.toml", &sysroot)));
    for item in fs::read_dir(format!("{}/etc/toolchains", &sysroot))
        .expect("couldn't find toolchains config dir ($JJS_SYSROOT/etc/jjs/toolchains")
    {
        let item = item.unwrap().path();
        let tc_cfg = fs::read_to_string(item).expect("Coudln't read toolchain config file");
        let raw_toolchain_spec_data: toml::Value = tc_cfg.parse().unwrap();
        let toolchain_spec: Toolchain = match toml::from_str(&tc_cfg) {
            Ok(x) => x,
            Err(e) => panic!(
                "Following error when parsing toolchain config: {:?}.\nRaw config:\n{:#?}",
                e, raw_toolchain_spec_data
            ),
        };
        c.toolchains.push(toolchain_spec);
    }
    c.sysroot = sysroot;
    c
}
