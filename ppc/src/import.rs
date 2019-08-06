mod template;

use std::{
    collections::{HashMap, HashSet},
    io::BufReader,
    path::{Path, PathBuf},
};
use xml::reader::XmlEvent;

struct Importer<'a> {
    src: &'a Path,
    dest: &'a Path,
    problem_cfg: crate::cfg::RawProblem,
    known_generators: HashSet<String>,
}

enum FileCategory {
    Validator,
    Checker,
    Generator,
}

impl FileCategory {
    fn derive(name: &str) -> Option<FileCategory> {
        if name == "check" || name == "checker" {
            return Some(FileCategory::Checker);
        }

        if name == "validator" {
            return Some(FileCategory::Validator);
        }

        if name.starts_with("gen") {
            return Some(FileCategory::Generator);
        }

        None
    }
}

impl<'a> Importer<'a> {
    fn event_as_string(&self, ev: XmlEvent) -> String {
        match ev {
            XmlEvent::Characters(str) => str,
            _ => {
                panic!("expected characters, got {:?}", ev);
            }
        }
    }

    fn read_tag_content_as_string(&self, iter: &mut impl Iterator<Item = XmlEvent>) -> String {
        let event = iter.next().unwrap();
        let data = self.event_as_string(event);
        let close_event = iter.next().unwrap();
        match &close_event {
            XmlEvent::EndElement { .. } => data,
            _ => panic!("expected end of tag, got {:?}", close_event),
        }
    }

    fn skip_section(&self, manifest: &mut impl Iterator<Item = XmlEvent>) {
        let mut depth = 1;
        while depth > 0 {
            match manifest.next().unwrap() {
                XmlEvent::StartElement { .. } => {
                    depth += 1;
                }
                XmlEvent::EndElement { .. } => {
                    depth -= 1;
                }
                _ => {}
            }
        }
    }

    fn parse_attrs(
        &self,
        attributes: Vec<xml::attribute::OwnedAttribute>,
    ) -> HashMap<String, String> {
        attributes
            .into_iter()
            .map(|x| (x.name.local_name, x.value))
            .collect::<HashMap<_, _>>()
    }

    // <problem><judging> is most important section for us: it contains information
    // about tests
    fn process_judging_section(&mut self, manifest: &mut impl Iterator<Item = XmlEvent>) {
        println!("<judging>");
        let testset_start = manifest.next().unwrap();
        match testset_start {
            XmlEvent::StartElement { name, .. } => {
                assert_eq!(name.local_name, "testset");
            }
            _ => {
                panic!("unexpected event: {:?}", testset_start);
            }
        }
        let mut memory_limit = None;
        let mut time_limit = None;
        let mut test_pattern = None;
        let mut ans_pattern = None;
        let mut test_count = None;
        loop {
            match manifest.next().unwrap() {
                XmlEvent::StartElement { name, .. } => match name.local_name.as_str() {
                    "time-limit" => {
                        let tl = self
                            .read_tag_content_as_string(manifest)
                            .parse::<u32>()
                            .expect("parsing <time-limit>:");
                        println!("time limit: {} ms", tl);
                        time_limit.replace(tl);
                    }
                    "memory-limit" => {
                        let ml = self
                            .read_tag_content_as_string(manifest)
                            .parse::<u32>()
                            .expect("parsing <memory-limit>:");
                        println!("memory limit: {} bytes ({} MiBs)", ml, ml / (1 << 20));
                        memory_limit.replace(ml);
                    }
                    "input-path-pattern" => {
                        let pat = self.read_tag_content_as_string(manifest);
                        println!("test input file path pattern: {}", &pat);
                        test_pattern.replace(pat);
                    }
                    "answer-path-pattern" => {
                        let pat = self.read_tag_content_as_string(manifest);
                        println!("test output file path pattern: {}", &pat);
                        ans_pattern.replace(pat);
                    }
                    "test-count" => {
                        let cnt = self
                            .read_tag_content_as_string(manifest)
                            .parse::<u32>()
                            .expect("parsing <test-count>:");
                        println!("test count: {}", cnt);
                        test_count.replace(cnt);
                    }
                    "tests" => {
                        self.process_tests(manifest);
                    }
                    _ => {
                        eprintln!(
                            "warning: unexpected tag in <problem><judging><testset>: {}",
                            name.local_name
                        );
                        self.skip_section(manifest);
                    }
                },
                XmlEvent::EndElement { name } => {
                    if name.local_name == "judging" {
                        break;
                    }
                }
                _ => continue,
            }
        }
        println!("</judging>");
    }

    fn import_file(&mut self, src_path: impl AsRef<Path>, dest_path: impl AsRef<Path>) {
        let full_src_path = self.src.join(src_path);
        let full_dest_path = self.dest.join(dest_path);
        match std::fs::copy(&full_src_path, &full_dest_path) {
            Ok(_) => {}
            Err(err) => {
                eprintln!(
                    "copy {} to {}: {}",
                    full_src_path.display(),
                    full_dest_path.display(),
                    err
                );
            }
        }
    }

    fn process_file(&mut self, file_path: &str, file_type: &str) {
        println!("processing {} of type {}", file_path, file_type);
        if !file_path.starts_with("files/") {
            eprintln!("file doesn't start from 'files/'.");
            return;
        }
        let file_name = file_path.trim_start_matches("files/");
        let period_pos = match file_name.find('.') {
            Some(p) => p,
            None => {
                eprintln!("file path does not contain extension");
                return;
            }
        };
        let file_name = &file_name[..period_pos];
        let category = match FileCategory::derive(file_name) {
            Some(cat) => cat,
            None => {
                if self.known_generators.contains(file_name) {
                    FileCategory::Generator
                } else {
                    eprintln!(
                        "couldn't derive file category (stripped name: {}).",
                        file_name
                    );
                    return;
                }
            }
        };
        match category {
            FileCategory::Validator => {
                // TODO
            }
            FileCategory::Checker => {
                // do nothing here, processed separately
            }
            FileCategory::Generator => {
                let module_dir = self.dest.join("modules").join(format!("gen-{}", file_name));
                std::fs::create_dir(&module_dir).expect("create module dir");
                let dest_path = module_dir.join("main.cpp");
                let src_path = self.src.join(file_path);
                std::fs::copy(&src_path, &dest_path).unwrap_or_else(|err| {
                    panic!(
                        "copy generator src from {} to {}: {}",
                        src_path.display(),
                        dest_path.display(),
                        err
                    );
                });

                let cmakefile = module_dir.join("CMakeLists.txt");
                // currently, CMakeLists are same with generator
                let cmakedata = template::get_checker_cmakefile(template::CheckerOptions {});
                std::fs::write(cmakefile, cmakedata).expect("write generator's CMakeLists.txt");
            }
        }
    }

    fn process_checker(&mut self, iter: &mut impl Iterator<Item = XmlEvent>) {
        println!("<checker>");
        loop {
            match iter.next().unwrap() {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    if name.local_name == "source" {
                        let attrs = self.parse_attrs(attributes);
                        let file_path = &attrs["path"];
                        self.import_file(
                            Path::new(file_path),
                            Path::new("modules/checker/main.cpp"),
                        );
                        let cmakefile = self.dest.join("modules/checker/CMakeLists.txt");
                        let cmakedata =
                            template::get_checker_cmakefile(template::CheckerOptions {});
                        std::fs::write(cmakefile, cmakedata)
                            .expect("write checker's CMakeLists.txt");
                    }
                    self.skip_section(iter);
                }
                XmlEvent::EndElement { name } => {
                    if name.local_name == "checker" {
                        break;
                    }
                }
                other => panic!("unexpected event: {:?}", other),
            }
        }
        println!("</checker>");
    }

    fn produce_generator_shim(&mut self) {
        {
            static SHIM: &str = include_str!("./import/gen-compat-shim.cpp");
            let dest_path = self.dest.join("testgens/shim/main.cpp");
            std::fs::write(dest_path, SHIM).expect("put generator-shim file");
        }
        {
            static SHIM_CMAKE: &str = include_str!("./import/shim.cmake");
            let dest_path = self.dest.join("testgens/shim/CMakeLists.txt");
            std::fs::write(dest_path, SHIM_CMAKE).expect("put generator-shim CMakeLists.txt");
        }
    }

    fn process_executables(&mut self, iter: &mut impl Iterator<Item = XmlEvent>) {
        println!("<executables>");
        loop {
            match iter.next().unwrap() {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    if name.local_name != "source" {
                        continue;
                    }
                    let attrs = self.parse_attrs(attributes);
                    self.process_file(attrs.get("path").unwrap(), attrs.get("type").unwrap());
                }
                XmlEvent::EndElement { name } => {
                    if name.local_name == "executables" {
                        break;
                    }
                }
                other => {
                    panic!("unexpected event: {:?}", other);
                }
            }
        }
        println!("</executables>");
    }

    fn process_files(&mut self, iter: &mut impl Iterator<Item = XmlEvent>) {
        println!("<files>");
        loop {
            match iter.next().unwrap() {
                XmlEvent::StartElement { name, .. } => match name.local_name.as_str() {
                    "resources" | "attachments" => {
                        self.skip_section(iter);
                    }
                    "executables" => {
                        self.process_executables(iter);
                    }
                    other => {
                        eprintln!("processing <files>: unexpected tag {}", other);
                    }
                },
                XmlEvent::EndElement { .. } => {
                    break;
                }
                other => {
                    panic!("processing <files>: unexpected event {:?}", other);
                }
            }
        }
        println!("</files>");
    }

    fn process_problem(&mut self, manifest: &mut impl Iterator<Item = XmlEvent>) {
        println!("<problem>");
        loop {
            match manifest.next().unwrap() {
                XmlEvent::StartElement { name, .. } => {
                    if name.local_name == "judging" {
                        self.process_judging_section(manifest);
                    } else if name.local_name == "names" {
                        self.process_names(manifest);
                    } else if name.local_name == "files" {
                        self.process_files(manifest);
                    } else if name.local_name == "assets" {
                        self.process_assets(manifest);
                    } else {
                        self.skip_section(manifest);
                    }
                }
                XmlEvent::EndElement { .. } => {
                    break;
                }
                _ => {}
            }
        }
        println!("</problem>");
    }

    fn process_tests(&mut self, iter: &mut impl Iterator<Item = XmlEvent>) {
        println!("<tests>");
        let mut cnt = 0;
        loop {
            match iter.next().unwrap() {
                XmlEvent::EndElement { name, .. } => {
                    if name.local_name == "tests" {
                        break;
                    }
                }
                XmlEvent::StartElement { attributes, .. } => {
                    cnt += 1;
                    let attrs = self.parse_attrs(attributes);
                    let mut ts = crate::cfg::RawTestsSpec {
                        map: cnt.to_string(),
                        testgen: None,
                        files: None,
                    };
                    let is_generated = attrs["method"] == "generated";
                    if is_generated {
                        let cmd_iter = attrs["cmd"].as_str().split_whitespace();
                        let mut testgen_cmd = cmd_iter.map(ToOwned::to_owned).collect::<Vec<_>>();
                        let gen_name = testgen_cmd[0].clone();
                        self.known_generators.insert(gen_name);
                        testgen_cmd.insert(0, "shim".to_string());
                        ts.testgen = Some(testgen_cmd);
                    } else {
                        ts.files = Some("{:0>2}.txt".to_string());
                        let src_path = format!("tests/{:0>2}", cnt);
                        let dest_path = format!("tests/{:0>2}.txt", cnt);
                        self.import_file(&src_path, &dest_path);
                    }
                    self.problem_cfg.tests.push(ts);
                }
                event => {
                    panic!("unexpected event: {:?}", event);
                }
            }
        }
        println!("{} tests imported", cnt);
        println!("</tests>");
    }

    fn process_solutions(&mut self, iter: &mut impl Iterator<Item = XmlEvent>) {
        println!("<solutions>");
        let mut last_tag = "".to_string();
        loop {
            match iter.next().unwrap() {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    let attrs = self.parse_attrs(attributes);
                    match name.local_name.as_str() {
                        "solution" => {
                            last_tag = attrs["tag"].clone();
                        }
                        "source" => {
                            if last_tag == "main" {
                                println!("importing main solution");
                                self.problem_cfg.primary_solution = Some("main".to_string());
                                let dir = self.dest.join("solutions/main");
                                {
                                    std::fs::create_dir_all(&dir)
                                        .expect("create main solution dir");
                                    let src_path = attrs["path"].clone();
                                    self.import_file(
                                        Path::new(&src_path),
                                        Path::new("solutions/main/main.cpp"),
                                    );
                                }
                                {
                                    let cmake_path = dir.join("CMakeLists.txt");
                                    let data = include_str!("./import/solution.cmake");
                                    std::fs::write(&cmake_path, data)
                                        .expect("write CMakeLists.txt for solution");
                                }
                            } else {
                                println!("skipping solution with tag {}: not main", &last_tag);
                            }

                            self.skip_section(iter);
                            self.skip_section(iter);
                        }
                        _ => {
                            panic!(
                                "unexpected tag {} when parsing <solutions>",
                                name.local_name
                            );
                        }
                    }
                }
                XmlEvent::EndElement { .. } => {
                    break;
                }
                other => panic!("unexpected event {:?}", other),
            }
        }
        println!("</solutions>")
    }

    fn process_assets(&mut self, iter: &mut impl Iterator<Item = XmlEvent>) {
        println!("<assets>");
        loop {
            match iter.next().unwrap() {
                XmlEvent::EndElement { name } => {
                    if name.local_name == "assets" {
                        break;
                    }
                }
                XmlEvent::StartElement { name, .. } => match name.local_name.as_str() {
                    "solutions" => {
                        self.process_solutions(iter);
                    }
                    "checker" => {
                        self.process_checker(iter);
                    }
                    _ => {
                        self.skip_section(iter);
                    }
                },
                other => panic!("unexpected event: {:?}", other),
            }
        }
        println!("</assets>");
    }

    fn process_names(&mut self, iter: &mut impl Iterator<Item = XmlEvent>) {
        println!("<names>");
        let ev = iter.next().unwrap();
        match ev {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                assert_eq!(name.local_name, "name");
                let attrs = self.parse_attrs(attributes);
                let title = attrs["value"].clone();
                self.problem_cfg.title = title.clone();
                println!("problem title: {}", &title);
                self.skip_section(iter);
            }
            _ => {
                panic!("parsing <names>: unexpected event {:?}", ev);
            }
        }
        self.skip_section(iter);
        println!("</names>");
    }

    fn fill_manifest(&mut self) {
        let m = &mut self.problem_cfg;
        m.valuer = "icpc".to_string();
        m.check_type = "builtin".to_string();
        m.builtin_check = Some(crate::cfg::BuiltinCheck {
            name: "polygon-compat".to_string(),
        });
        m.check_options = Some(crate::cfg::CheckOptions {
            args: vec!["assets/module-checker/bin".to_string()],
        });
        let mut random_seed = [0; 32];
        getrandom::getrandom(&mut random_seed).unwrap();
        let rand_seed_hex = hex::encode(&random_seed);
        m.random_seed = Some(rand_seed_hex);
    }

    fn init_dirs(&mut self) {
        for suf in &[
            "solutions",
            "testgens",
            "testgens/shim",
            "tests",
            "modules",
            "modules/checker",
        ] {
            let path = self.dest.join(suf);
            std::fs::create_dir(&path)
                .unwrap_or_else(|err| panic!("create {}: {}", path.display(), err));
        }

        // import testlib
        self.import_file(Path::new("files/testlib.h"), Path::new("testlib.h"));
    }

    fn run(&mut self) {
        let manifest_path = self.src.join("problem.xml");
        let manifest = std::fs::File::open(&manifest_path).unwrap_or_else(|err| {
            panic!(
                "error: open manifest at {}: {}",
                manifest_path.display(),
                err
            );
        });
        let manifest = BufReader::new(manifest);
        let mut parser = xml::EventReader::new(manifest);
        let mut event_iter =
            std::iter::from_fn(move || Some(parser.next().expect("reading manifest"))).filter(
                |ev| match ev {
                    XmlEvent::Whitespace(_) => false,
                    _ => true,
                },
            );
        self.init_dirs();
        self.fill_manifest();
        self.produce_generator_shim();
        loop {
            let event = event_iter.next().unwrap();
            if let XmlEvent::StartElement {
                name, attributes, ..
            } = event
            {
                assert_eq!(name.local_name, "problem");
                let attrs = self.parse_attrs(attributes);
                self.problem_cfg.name = attrs
                    .get("short-name")
                    .map(|x| x.to_string())
                    .expect("missing short-name in <problem>");
                self.process_problem(&mut event_iter);
                return;
            }
        }
    }
}

pub fn exec(args: crate::args::ImportArgs) {
    if args.force {
        std::fs::remove_dir_all(&args.out_path).expect("remove out dir");
        std::fs::create_dir(&args.out_path).expect("recreate out dir")
    } else {
        crate::check_dir(&PathBuf::from(&args.out_path), false /* TODO */);
    }

    let src = PathBuf::from(&args.external_package);
    let dest = PathBuf::from(&args.out_path);

    let mut importer = Importer {
        src: &src,
        dest: &dest,
        problem_cfg: Default::default(),
        known_generators: HashSet::new(),
    };

    importer.run();

    let manifest_path = dest.join("problem.toml");
    let manifest_toml =
        toml::Value::try_from(importer.problem_cfg.clone()).expect("serialize ppc config");
    let manifest_data = toml::ser::to_string_pretty(&manifest_toml).unwrap_or_else(|err| {
        panic!(
            "stringify ppc config: {}\n\nraw config: {:#?}",
            err, &importer.problem_cfg
        )
    });
    std::fs::write(manifest_path, manifest_data).expect("write ppc manifest");
}
