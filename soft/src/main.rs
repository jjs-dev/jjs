use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    /// Where to put files
    #[structopt(default_value = "/", long = "root", short = "r")]
    root: String,
    /// What toolchains to search
    #[structopt(long = "with")]
    with: Vec<String>,
    /// What debs to install
    #[structopt(long = "deb")]
    deb: Vec<String>,
}

#[derive(Clone, Copy)]
struct FindArgs<'a> {
    //TODO: root: &'a str,
    target: &'a str,
}

fn deduce_interpreter(path: &str) -> Option<String> {
    //TODO: check shebang
    let file_output = Command::new("file")
        .arg("--dereference") //follow symlinks
        .arg(path)
        .stderr(Stdio::inherit())
        .stdin(Stdio::null())
        .output()
        .expect("Couldn't describe");

    assert!(file_output.status.success());
    let info = String::from_utf8_lossy(&file_output.stdout).to_string();
    let info = info.split_whitespace();
    let interp = info.skip_while(|t| *t != "interpreter").nth(1);
    interp
        .map(std::string::ToString::to_string)
        .map(|s| s.replace(',', ""))
}

fn find_binary(args: FindArgs, bin_name: &str) {
    let full_path = Command::new("which")
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .arg(bin_name)
        .output()
        .unwrap_or_else(|e| panic!("Couldn't resolve path to {}: error {}", bin_name, e));
    assert_eq!(full_path.status.success(), true);
    let full_path = String::from_utf8(full_path.stdout)
        .expect("Couldn't parse utf8")
        .trim()
        .to_string();
    let ldd = Command::new("ldd")
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .arg(&full_path)
        .output()
        .unwrap_or_else(|e| panic!("Couldn't get dependencies of {}: error {}", full_path, e));
    let mut has_deps = true;
    let ldd_output = String::from_utf8(ldd.stdout).expect("Couldn't parse utf8");

    if ldd_output.contains("not a dynamic executable") {
        has_deps = false;
    }
    let base_files = [full_path.clone()];
    let mut files: Vec<String> = base_files.to_vec();
    if has_deps {
        assert!(ldd.status.success());
        let deps = ldd_output
            .split('\n')
            .filter_map(|line| line.split("=>").nth(1))
            .filter_map(|x| x.split_whitespace().nth(0))
            .map(std::string::ToString::to_string);
        let interp = deduce_interpreter(full_path.as_str());
        if let Some(interp) = interp {
            files.push(interp);
        }
        for x in deps {
            files.push(x);
        }
    }
    let mut options = fs_extra::dir::CopyOptions::new();
    options.skip_exist = true;
    for item in files {
        let path = PathBuf::from(&item);
        let base_path = path.parent().unwrap().to_str().unwrap();
        let resulting_path = format!("{}{}", args.target, base_path);
        fs::create_dir_all(&resulting_path).ok();
        fs_extra::copy_items(&vec![item], resulting_path, &options)
            .expect("Couldn't copy binary with its dependencies");
    }
}

trait CommandExt {
    fn exec(&mut self);
}

impl CommandExt for std::process::Command {
    fn exec(&mut self) {
        let output = self.output().expect("Couldn't execute");
        let out = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        println!("{}", out);
    }
}

fn setup_dpkg(dir: &str) {
    for path in ["/var/lib/dpkg/updates", "/var/lib/dpkg/info", "/var/lib/dpkg/tmp.ci"].iter() {
        fs::create_dir_all(format!("{}/{}", dir, path)).expect("Couldn't create dir");
    }
    fs::File::create(format!("{}/var/lib/dpkg/status", dir)).unwrap();
}

fn resolve_dependencies(pkg: &str) -> Vec<String> {
    let out = Command::new("apt-cache")
        .arg("depends")
        .arg(pkg)
        .output().expect("Couldn't query dependencies");
    let out = String::from_utf8_lossy(&out.stdout).to_string();
    let out = out.split('\n').skip(1);
    let mut res = Vec::new();
    for line in out {
        let t: Vec<_> = line.split(' ').map(|x| x.trim().to_string()).filter(|x| x.len() > 0).collect();
        if t.is_empty() {
            continue;
        }
        if t.len() != 2 {
            eprintln!("warning: skipping line {} of unknown format", &line);
            continue;
        }
        let mut relation: String = t[0].clone();
        let package: String = t[1].clone();
        relation.pop().expect("parse error");
        if relation == "Depends" {
            res.push(package);
        }
    };
    res
}

fn resolve_dependencies_recursive(start: &[String]) -> Vec<String> {
    //just BFS
    let mut queue = std::collections::VecDeque::<String>::new();
    let mut visited = std::collections::HashSet::<String>::new();
    for pkg in start {
        queue.push_back(pkg.clone());
    }
    while !queue.is_empty() {
        let pkg = queue.pop_front().unwrap();
        visited.insert(pkg.clone());
        let deps = resolve_dependencies(&pkg);
        for dep in &deps {
            if !visited.contains(dep.as_str()) {
                queue.push_back(dep.clone());
                visited.insert(dep.clone());
            }
        }
    }

    visited.into_iter().collect()
}

fn fetch_debian_package(pkg: &str) -> String {
    println!("adding package {}", pkg);
    let workdir = "/tmp/jjs-soft-debs";
    fs::create_dir(workdir).ok();
    Command::new("apt")
        .current_dir(workdir)
        .arg("download")
        .arg(pkg)
        .exec();
    let pat = format!("{}/{}_*.deb", workdir, pkg);
    let items: Vec<_> = glob::glob(&pat).expect("Couldn't search for package").collect();
    if items.is_empty() {
        panic!("Package not found");
    }
    if items.len() > 1 {
        panic!("More than one candidate is found: {:?}", items);
    }
    let item = items.into_iter().next().unwrap().expect("io error");
    let item = item.to_str().unwrap();
    item.to_string()
}

fn add_debian_packages(pkg: &[&str], dir: &str) {
    let mut cmd = Command::new("dpkg");
    cmd
        .arg("-i")
        .arg("--force-not-root")
        .arg(format!("--root={}", dir));

    for &p in pkg {
        cmd.arg(p);
    }
    cmd.exec();
}


fn main() {
    let opt: Options = Options::from_args();
    let arg = FindArgs { target: &opt.root };
    for bin in opt.with {
        find_binary(arg, bin.as_str());
    }
    if !opt.deb.is_empty() {
        println!("installing selected debs ({} requested)", opt.deb.len());
        setup_dpkg(&opt.root);

        let input_packages = opt.deb.clone();
        let packages_with_deps = resolve_dependencies_recursive(&input_packages);
        let mut archives = Vec::new();
        println!("installing packages: {:?}", &archives);
        for p in &packages_with_deps {
            let saved_path = fetch_debian_package(p);
            archives.push(saved_path);
        }
        let debs: Vec<&str> = archives.iter().map(|x| x.as_str()).collect();
        add_debian_packages(debs.as_slice(), &opt.root);
    }
}
