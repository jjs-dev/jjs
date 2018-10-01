extern crate serde;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use std::{
    collections::{
        HashMap,
    },
    path::{
        PathBuf
    },
    fs,
    env,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Command {
    pub env: HashMap<String, String>,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Toolchain {
    pub name: String,
    pub extension: String,
    pub build_commands: Vec<Command>,
    pub run_command: Command,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub toolchains: Vec<Toolchain>,
    #[serde(skip_deserializing)]
    pub sysroot: PathBuf,
}

pub fn parse_file(path: PathBuf) -> Config  {
    let file_content = fs::read_to_string(path).unwrap();
    let raw_data :toml::Value =  file_content.parse().unwrap() ;
    match toml::from_str(&file_content){
        Ok(x) => x,
        Err(e) => {
            panic!("Error ocured when parsing config: {:?}.\nRaw config:\n{:#?}", e, raw_data)
        }
    }
}

pub fn get_config() -> Config {
    let args :Vec<_>= env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} path/to/sysroot", args[0]);
        std::process::exit(1);
    }
    let mut c = parse_file(PathBuf::from(format!("{}/etc/jjs/jjs.toml", &args[1])));
    c.sysroot = PathBuf::from(args[1].clone());
    c
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
