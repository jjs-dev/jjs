extern crate serde;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use std::{collections::HashMap, env, fs, path::PathBuf};

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct Toolchain {
    pub name: String,
    pub filename: String,
    #[serde(rename = "build")]
    pub build_commands: Vec<Command>,
    #[serde(rename = "run")]
    pub run_command: Command,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Problem {
    pub name: String,
    pub code: String,
    #[serde(skip)]
    pub title: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Contest {
    pub title: String,
    /// Information about problems, not related to judging
    /// process (which is controlled by problem itself)
    pub problems: Vec<Problem>,
    /// Whether contest is visible for users that are not included in contestants
    #[serde(rename = "vis-unreg")]
    pub unregistered_visible: bool,
    /// Whether contest is visible for anonymous users
    #[serde(rename = "vis-anon")]
    pub anon_visible: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub toolchains: Vec<Toolchain>,
    #[serde(skip)]
    pub sysroot: String,
    #[serde(rename = "toolchain-root")]
    pub toolchain_root: String,
    #[serde(rename = "global-limits")]
    pub global_limits: Limits,
    #[serde(rename = "global-env")]
    pub global_env: HashMap<String, String>,
    #[serde(rename = "env-passing")]
    pub env_passing: bool,
    #[serde(skip)]
    pub contests: Vec<Contest>,
}

impl Config {
    pub fn postprocess(&mut self) {
        fn command_inherit_env(cmd: &mut Command, dfl: &HashMap<String, String>) {
            for (key, val) in dfl.iter() {
                cmd.env.entry(key.clone()).or_insert_with(|| val.clone());
            }
        }

        let mut inherit_env = self.global_env.clone();
        if self.env_passing {
            for (key, value) in std::env::vars() {
                inherit_env.entry(key).or_insert(value);
            }
        }

        for toolchain in &mut self.toolchains {
            for mut cmd in &mut toolchain.build_commands {
                command_inherit_env(&mut cmd, &inherit_env);
            }
            command_inherit_env(&mut toolchain.run_command, &inherit_env);
        }
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
    let cfg: Config = match toml::from_str(&file_content) {
        Ok(x) => x,
        Err(e) => panic!(
            "Error ocured when parsing config: {:?}.\nRaw config:\n{:#?}",
            e, raw_data
        ),
    };
    cfg
}

pub fn get_config() -> Config {
    let sysroot = env::var("JJS_SYSROOT").expect("Sysroot must be provided in JJS_SYSROOT");
    let mut c = parse_file(PathBuf::from(format!("{}/etc/jjs.toml", &sysroot)));
    // load toolchains
    for item in fs::read_dir(format!("{}/etc/toolchains", &sysroot))
        .expect("couldn't find toolchains config dir (JJS_SYSROOT/etc/jjs/toolchains")
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
    // load contests
    // TODO: support multiple contests
    {
        let contest_cfg_path = format!("{}/etc/contest.toml", &sysroot);
        let contest_cfg = fs::read_to_string(contest_cfg_path).expect("failed read contest config");
        let mut contest: Contest = toml::from_str(&contest_cfg).expect("failed parse contest");
        for problem in contest.problems.iter_mut() {
            let problem_manifest_path =
                format!("{}/var/problems/{}/manifest.json", &sysroot, &problem.name);
            let problem_manifest: pom::Problem = serde_json::from_reader(
                fs::File::open(problem_manifest_path).expect("failed read problem manifest"),
            )
            .unwrap();
            problem.title = problem_manifest.title;
        }
        c.contests.push(contest);
    }
    c.sysroot = sysroot;
    c.postprocess();
    c
}
