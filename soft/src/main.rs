use std::{collections::HashSet, path::PathBuf};
use structopt::StructOpt;
#[derive(Debug)]
enum OutType {
    Text,
    Json,
}

impl OutType {
    fn strcutopt_parse(inp: &str) -> Result<Self, &'static str> {
        match inp.to_lowercase().as_str() {
            "text" => Ok(OutType::Text),
            "json" => Ok(OutType::Json),
            _ => Err("unknown type"),
        }
    }
}

impl std::str::FromStr for OutType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::strcutopt_parse(s)
    }
}

#[derive(Debug, StructOpt)]
struct Options {
    /// JSON log file to analyze
    #[structopt(long = "data", short = "i")]
    log_file: String,
    /// Where to put result
    #[structopt(long = "dest", short = "d")]
    dest_file: String,
    /// Format (json or text)
    #[structopt(long = "format", short = "f")]
    format: OutType,
    /// Path prefix to skip (e.g., /home)
    #[structopt(long = "skip", short = "s")]
    skip: Vec<String>,
    /// Disable default skip  prefixes
    #[structopt(long = "no-default-skip-list", short = "k")]
    no_def_skip: bool,
    /// Resolve all paths from given dir (default is cwd)
    #[structopt(long = "resolve-dir", short = "r", default_value = ".")]
    resolve_dir: String,
}

impl Options {
    fn preprocess(&mut self) {
        if !self.no_def_skip {
            self.skip.push("/tmp".to_string());
            self.skip.push("/proc".to_string());
            self.skip.push("/dev".to_string());
        }
    }
}

trait ResultExt {
    type Ok;
    fn conv(self) -> Result<Self::Ok, ()>;
}

impl<T, E> ResultExt for Result<T, E> {
    type Ok = T;
    fn conv(self) -> Result<T, ()> {
        self.map_err(|_err| ())
    }
}

impl<T> ResultExt for Option<T> {
    type Ok = T;
    fn conv(self) -> Result<T, ()> {
        self.ok_or(())
    }
}

fn process_log_item(value: &serde_json::Value, out: &mut HashSet<String>) -> Result<(), ()> {
    let value = value.as_object().conv()?;
    let syscall_name = value.get("syscall").conv()?.as_str().conv()?.to_owned();
    let syscall_args = value.get("args").conv()?.as_array().conv()?;
    let syscall_ret = value.get("result").conv()?.as_str().unwrap_or("");
    if syscall_ret.find('-').is_some()
        && syscall_ret.find('E').is_some()
        && syscall_ret.find('(').is_some()
    {
        // syscall likely to be failed
        return Ok(());
    }

    match syscall_name.as_str() {
        "execve" => {
            //read argv[0]
            out.insert(syscall_args[0].as_str().conv()?.to_owned());
        }
        "access" => {
            //accessed file
            out.insert(syscall_args[1].as_str().conv()?.to_owned());
        }
        "openat" => {
            // opened file
            out.insert(syscall_args[1].as_str().conv()?.to_owned());
        }
        "open" => {
            // opened file
            out.insert(syscall_args[0].as_str().conv()?.to_owned());
        }
        _ => {}
    }

    Ok(())
}

fn filter_file_name(name: &str, skip_list: &[&str]) -> bool {
    for sk in skip_list {
        if name.starts_with(sk) {
            return false;
        }
    }
    true
}

fn normalize_path(path: PathBuf, root: PathBuf) -> Option<PathBuf> {
    if path.is_absolute() {
        return Some(path);
    }
    let path = root.join(path);

    let mut out = PathBuf::new();

    for item in path.into_iter() {
        if item == "." {
            continue;
        } else if item == ".." {
            if !out.pop() {
                return None;
            }
        } else {
            out.push(item);
        }
    }

    dbg!(&out);

    Some(out)
}

fn main() {
    let mut opt: Options = Options::from_args();
    opt.preprocess();
    let data = std::fs::read_to_string(&opt.log_file).expect("couldn't open log");
    let values: Vec<serde_json::Value> = serde_json::Deserializer::from_str(&data)
        .into_iter()
        .map(|x| x.unwrap())
        .collect();
    println!(
        "got {} bytes, {} entries",
        data.as_bytes().len(),
        values.len()
    );
    let mut out = HashSet::new();
    for value in values.iter() {
        process_log_item(value, &mut out).ok() /*ignore possible errors, we are best-effort*/;
    }
    let skip_list: Vec<_> = opt.skip.iter().map(|s| s.as_str()).collect();
    
    let resolve_dir= std::fs::canonicalize(&opt.resolve_dir).unwrap();

    let mut files: Vec<_> = out
        .into_iter()
        .filter_map(|file| {
            use std::str::FromStr;
            normalize_path(
                PathBuf::from_str(&file).unwrap(),
                resolve_dir.clone(),
            )
            .map(|x| x.to_str().unwrap().to_string())
        })
        .filter(|file| filter_file_name(file, &skip_list))
        .collect();
    files.sort();
    println!("{} files found", files.len());

    let out_data: String;
    match opt.format {
        OutType::Json => {
            out_data = serde_json::to_string(&files).unwrap();
        }
        OutType::Text => out_data = (&files).join("\n"),
    }
    std::fs::write(opt.dest_file, out_data).unwrap();
}
