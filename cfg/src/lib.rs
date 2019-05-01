extern crate serde;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use std::{collections::HashMap, env, fs, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Limits {
    #[serde(default = "Limits::default_time")]
    pub memory: u64,
    #[serde(default = "Limits::default_memory")]
    pub time: u64,
    #[serde(default = "Limits::default_num_procs")]
    pub process_count: u64,
}

impl Limits {
    fn default_num_procs() -> u64 {
        16
    }

    fn default_memory() -> u64 {
        256 * 1024
    }

    fn default_time() -> u64 {
        1000
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Toolchain {
    pub name: String,
    pub filename: String,
    #[serde(rename = "build")]
    pub build_commands: Vec<Command>,
    #[serde(rename = "run")]
    pub run_command: Command,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub toolchains: Vec<Toolchain>,
    #[serde(skip)]
    pub sysroot: String,
    #[serde(rename = "toolchain-root")]
    pub toolchain_root: String,
    #[serde(rename = "global-limits")]
    pub global_limits: Limits,
}

impl Config {
    pub fn postprocess(&mut self) {
        //TODO
    }
}

pub fn parse_file(path: PathBuf) -> Config {
    let file_content = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "Couldn't read main config file {:?} due to error {:?}",
            &path, e
        )
    });
    let raw_data: toml::Value = file_content.parse().unwrap();
    let mut cfg: Config = match toml::from_str(&file_content) {
        Ok(x) => x,
        Err(e) => panic!(
            "Error ocured when parsing config: {:?}.\nRaw config:\n{:#?}",
            e, raw_data
        ),
    };
    cfg.postprocess();
    cfg
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
