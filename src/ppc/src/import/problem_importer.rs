use anyhow::Context as _;
use std::{collections::HashSet, path::Path};

pub(crate) struct Importer<'a> {
    pub(crate) src: &'a Path,
    pub(crate) dest: &'a Path,
    pub(crate) problem_cfg: crate::manifest::RawProblem,
    pub(crate) known_generators: HashSet<String>,
    pub(crate) doc: roxmltree::Node<'a, 'a>,
    pub(crate) limits: pom::Limits,
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
    // <problem><judging> is most important section for us: it contains information
    // about tests
    fn process_judging_section(&mut self, node_judging: roxmltree::Node) -> anyhow::Result<()> {
        let node_testset = node_judging.first_element_child().unwrap();
        assert_eq!(node_testset.tag_name().name(), "testset");

        let mut test_pattern = None;
        let mut ans_pattern = None;
        for child in node_testset.children() {
            if !child.is_element() {
                continue;
            }
            match child.tag_name().name() {
                "time-limit" => {
                    let tl = child
                        .text()
                        .unwrap()
                        .parse::<u64>()
                        .expect("parsing <time-limit>:");
                    println!("time limit: {} ms", tl);
                    self.limits.time.replace(tl);
                }
                "memory-limit" => {
                    let ml = child
                        .text()
                        .unwrap()
                        .parse::<u64>()
                        .expect("parsing <memory-limit>:");
                    println!("memory limit: {} bytes ({} MiBs)", ml, ml / (1 << 20));
                    self.limits.memory.replace(ml);
                }
                "input-path-pattern" => {
                    let pat = child.text().unwrap().to_string();
                    println!("test input file path pattern: {}", &pat);
                    test_pattern.replace(pat);
                }
                "answer-path-pattern" => {
                    let pat = child.text().unwrap().to_string();
                    println!("test output file path pattern: {}", &pat);
                    ans_pattern.replace(pat);
                }
                "test-count" => {}
                "tests" => {
                    self.process_tests(child)?;
                }
                _ => {
                    eprintln!(
                        "warning: unexpected tag in <problem><judging><testset>: {}",
                        child.tag_name().name()
                    );
                }
            }
        }
        Ok(())
    }

    fn import_file(
        &mut self,
        src_path: impl AsRef<Path>,
        dest_path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let full_src_path = self.src.join(src_path);
        let full_dest_path = self.dest.join(dest_path);
        std::fs::copy(&full_src_path, &full_dest_path)
            .with_context(|| {
                format!(
                    "copy {} to {}",
                    full_src_path.display(),
                    full_dest_path.display()
                )
            })
            .map(drop)
    }

    fn process_file(&mut self, file_path: &str, file_type: &str) -> anyhow::Result<()> {
        println!("processing {} of type {}", file_path, file_type);
        if !file_path.starts_with("files/") {
            eprintln!("file doesn't start from 'files/'.");
            return Ok(());
        }
        let file_name = file_path.trim_start_matches("files/");
        let period_pos = match file_name.find('.') {
            Some(p) => p,
            None => {
                eprintln!("file path does not contain extension");
                return Ok(());
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
                    return Ok(());
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
                std::fs::copy(&src_path, &dest_path).with_context(|| {
                    format!(
                        "copy generator src from {} to {}",
                        src_path.display(),
                        dest_path.display()
                    )
                })?;

                let cmakefile = module_dir.join("CMakeLists.txt");
                // currently, CMakeLists are same with generator
                let cmakedata =
                    super::template::get_checker_cmakefile(super::template::CheckerOptions {});
                std::fs::write(cmakefile, cmakedata).context("write generator's CMakeLists.txt")?;
            }
        }
        Ok(())
    }

    fn process_checker(&mut self, node_checker: roxmltree::Node) -> anyhow::Result<()> {
        println!("Importing checker");
        assert_eq!(node_checker.attribute("type"), Some("testlib"));
        for child in node_checker.children() {
            if !child.is_element() {
                continue;
            }
            if child.tag_name().name() != "source" {
                continue;
            }
            let file_path = child.attribute("path").unwrap();
            self.import_file(Path::new(file_path), Path::new("modules/checker/main.cpp"))?;
            let cmakefile = self.dest.join("modules/checker/CMakeLists.txt");
            let cmakedata =
                super::template::get_checker_cmakefile(super::template::CheckerOptions {});
            std::fs::write(cmakefile, cmakedata).context("write checker's CMakeLists.txt")?;
        }
        Ok(())
    }

    fn produce_generator_shim(&mut self) {
        {
            static SHIM: &str = include_str!("./gen-compat-shim.cpp");
            let dest_path = self.dest.join("testgens/shim/main.cpp");
            std::fs::write(dest_path, SHIM).expect("put generator-shim file");
        }
        {
            static SHIM_CMAKE: &str = include_str!("./shim.cmake");
            let dest_path = self.dest.join("testgens/shim/CMakeLists.txt");
            std::fs::write(dest_path, SHIM_CMAKE).expect("put generator-shim CMakeLists.txt");
        }
    }

    fn process_executable(&mut self, node_executable: roxmltree::Node) -> anyhow::Result<()> {
        for node_source in node_executable.children() {
            if node_source.tag_name().name() != "source" {
                continue;
            }
            let attr_path = node_source.attribute("path").unwrap();
            let attr_type = node_source.attribute("type").unwrap();
            self.process_file(attr_path, attr_type)?;
        }
        Ok(())
    }

    fn process_tests(&mut self, tests_node: roxmltree::Node) -> anyhow::Result<()> {
        println!("Importing tests");
        assert_eq!(tests_node.tag_name().name(), "tests");
        let mut cnt = 0;
        for test_node in tests_node.children() {
            if !test_node.is_element() {
                continue;
            }
            assert_eq!(test_node.tag_name().name(), "test");
            cnt += 1;
            let mut ts = crate::manifest::RawTestsSpec {
                map: cnt.to_string(),
                testgen: None,
                files: None,
                limits: self.limits,
                group: test_node
                    .attribute("group")
                    .unwrap_or("default")
                    .to_string(),
            };
            let is_generated = test_node.attribute("method").unwrap() == "generated";
            if is_generated {
                let cmd_iter = test_node.attribute("cmd").unwrap().split_whitespace();
                let mut testgen_cmd = cmd_iter.map(ToOwned::to_owned).collect::<Vec<_>>();
                let gen_name = testgen_cmd[0].clone();
                self.known_generators.insert(gen_name);
                testgen_cmd.insert(0, "shim".to_string());
                ts.testgen = Some(testgen_cmd);
            } else {
                // TODO: use formatf here instead of hardcoded format strings
                ts.files = Some("%02d.txt".to_string());
                let src_path = format!("tests/{:0>2}", cnt);
                let dest_path = format!("tests/{:0>2}.txt", cnt);
                self.import_file(&src_path, &dest_path)?;
            }
            self.problem_cfg.tests.push(ts);
        }
        println!("{} tests imported", cnt);
        Ok(())
    }

    fn process_solutions(&mut self, node: roxmltree::Node) -> anyhow::Result<()> {
        println!("Importing solution");
        for solution_node in node.children() {
            if !solution_node.is_element() {
                continue;
            }
            let tag = solution_node.attribute("tag").unwrap();
            if tag == "main" {
                println!("importing main solution");
                self.problem_cfg.primary_solution = Some("main".to_string());
                let dir = self.dest.join("solutions/main");
                let mut src_path = None;
                for child in solution_node.children() {
                    if !child.is_element() {
                        continue;
                    }
                    if child.tag_name().name() == "source" {
                        src_path = Some(child.attribute("path").unwrap());
                    }
                }
                let src_path = src_path.unwrap();
                std::fs::create_dir_all(&dir).context("create main solution dir")?;
                self.import_file(Path::new(&src_path), Path::new("solutions/main/main.cpp"))?;
                {
                    let cmake_path = dir.join("CMakeLists.txt");
                    let data = include_str!("./solution.cmake");
                    std::fs::write(&cmake_path, data)
                        .context("write CMakeLists.txt for solution")?;
                }
            } else {
                println!("skipping solution with tag {}: not main", &tag);
            }
        }
        Ok(())
    }

    fn process_names(&mut self, node_names: roxmltree::Node) {
        println!("Importing name");
        assert!(node_names.is_element());
        for child in node_names.children() {
            if !child.is_element() {
                continue;
            }
            let title = child.attribute("value").unwrap();

            self.problem_cfg.title = title.to_string();
            println!("problem title: {}", &title);
            return;
        }
    }

    fn fill_manifest(&mut self) -> anyhow::Result<()> {
        let m = &mut self.problem_cfg;
        m.valuer = "icpc".to_string();
        m.check_type = "builtin".to_string();
        m.builtin_check = Some(crate::manifest::BuiltinCheck {
            name: "polygon-compat".to_string(),
        });
        m.check_options = Some(crate::manifest::CheckOptions {
            args: vec!["assets/module-checker/bin".to_string()],
        });
        let mut random_seed = [0; 32];
        getrandom::getrandom(&mut random_seed)?;
        let rand_seed_hex = hex::encode(&random_seed);
        m.random_seed = Some(rand_seed_hex);
        Ok(())
    }

    fn init_dirs(&mut self) -> anyhow::Result<()> {
        for suf in &[
            "solutions",
            "testgens",
            "testgens/shim",
            "tests",
            "modules",
            "modules/checker",
        ] {
            let path = self.dest.join(suf);
            std::fs::create_dir(&path).with_context(|| format!("create {}", path.display()))?;
        }

        // import testlib
        self.import_file(Path::new("files/testlib.h"), Path::new("testlib.h"))?;
        Ok(())
    }

    fn feed(&mut self, node: roxmltree::Node) -> anyhow::Result<()> {
        match node.tag_name().name() {
            "names" => self.process_names(node),
            "solutions" => self.process_solutions(node)?,
            "judging" => self.process_judging_section(node)?,
            "executable" => self.process_executable(node)?,
            "checker" => self.process_checker(node)?,
            _ => {
                for ch in node.children() {
                    self.feed(ch)?;
                }
            }
        }
        Ok(())
    }

    fn import_valuer_config(&mut self) -> anyhow::Result<()> {
        let valuer_cfg_path = self.src.join("files/valuer.cfg");
        let config;
        if valuer_cfg_path.exists() {
            println!("Importing valuer.cfg from {}", valuer_cfg_path.display());
            config = serde_yaml::to_string(&super::valuer_cfg::import(&valuer_cfg_path)?)?;
        } else {
            config = include_str!("./default_valuer_config.yaml").to_string();
        }
        std::fs::write(self.dest.join("valuer.yaml"), config)?;
        Ok(())
    }

    pub(crate) fn run(&mut self) -> anyhow::Result<()> {
        self.init_dirs()?;
        self.fill_manifest()?;
        self.produce_generator_shim();
        self.feed(self.doc)?;
        self.import_valuer_config()?;
        Ok(())
    }
}
