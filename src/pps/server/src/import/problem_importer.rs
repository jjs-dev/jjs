use anyhow::Context as _;
use pps_api::{
    import_problem::{PropertyName, Update},
    SimpleFinish,
};
use std::{collections::HashSet, future::Future, path::Path, pin::Pin};

pub(crate) struct Importer<'a> {
    pub(crate) src: &'a Path,
    pub(crate) dest: &'a Path,
    pub(crate) problem_cfg: crate::manifest::RawProblem,
    pub(crate) known_generators: HashSet<String>,
    pub(crate) doc: roxmltree::Node<'a, 'a>,
    pub(crate) limits: pom::Limits,
    pub(crate) tx: &'a mut rpc::StreamingTx<Update, SimpleFinish>,
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
    async fn process_judging_section(
        &mut self,
        node_judging: roxmltree::Node<'_, '_>,
    ) -> anyhow::Result<()> {
        let node_testset = node_judging
            .first_element_child()
            .context("<judging> element does not have a <testset> child")?;
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
                        .context("<time-limit> does not contain value")?
                        .parse::<u64>()
                        .context("parsing <time-limit> failed")?;
                    self.tx
                        .send_event(Update::Property {
                            property_name: PropertyName::TimeLimit,
                            property_value: tl.to_string(),
                        })
                        .await?;
                    self.limits.time.replace(tl);
                }
                "memory-limit" => {
                    let ml = child
                        .text()
                        .context("<memory-limiy> does not contain value")?
                        .parse::<u64>()
                        .context("parsing <memory-limit> failed")?;
                    self.tx
                        .send_event(Update::Property {
                            property_name: PropertyName::MemoryLimit,
                            property_value: ml.to_string(),
                        })
                        .await?;
                    self.limits.memory.replace(ml);
                }
                "input-path-pattern" => {
                    let pat = child.text().unwrap().to_string();
                    self.tx
                        .send_event(Update::Property {
                            property_name: PropertyName::InputPathPattern,
                            property_value: pat.clone(),
                        })
                        .await?;
                    test_pattern.replace(pat);
                }
                "answer-path-pattern" => {
                    let pat = child.text().unwrap().to_string();
                    self.tx
                        .send_event(Update::Property {
                            property_name: PropertyName::OutputPathPattern,
                            property_value: pat.clone(),
                        })
                        .await?;
                    ans_pattern.replace(pat);
                }
                "test-count" => {}
                "tests" => {
                    self.process_tests(child).await?;
                }
                _ => {
                    let message = format!(
                        "unexpected tag in <problem><judging><testset>: {}",
                        child.tag_name().name()
                    );
                    self.tx.send_event(Update::Warning(message)).await?;
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

    async fn process_file(&mut self, file_path: &str, file_type: &str) -> anyhow::Result<()> {
        if !file_path.starts_with("files/") {
            return Ok(());
        }
        let file_name = file_path.trim_start_matches("files/");
        let period_pos = match file_name.find('.') {
            Some(p) => p,
            None => {
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
                    let message = format!(
                        "couldn't derive file category (stripped name: {})",
                        file_name
                    );
                    self.tx.send_event(Update::Warning(message)).await?;

                    return Ok(());
                }
            }
        };
        match category {
            FileCategory::Validator => {
                let message = "ignoring validators: not yet implemented".to_string();
                self.tx.send_event(Update::Warning(message)).await?;
            }
            FileCategory::Checker => {
                // do nothing here, processed separately
            }
            FileCategory::Generator => {
                let gen_dir = self.dest.join("generators").join(file_name);
                tokio::fs::create_dir(&gen_dir)
                    .await
                    .expect("create generator dir");
                let extension = match file_type {
                    _ if file_type.starts_with("cpp.g++") => "cpp",
                    "python.3" => "py",
                    _ => anyhow::bail!("unknown file type: {}", file_type),
                };
                let dest_path = gen_dir.join(format!("main.{}", extension));
                let src_path = self.src.join(file_path);
                tokio::fs::copy(&src_path, &dest_path)
                    .await
                    .with_context(|| {
                        format!(
                            "copy generator src from {} to {}",
                            src_path.display(),
                            dest_path.display()
                        )
                    })?;

                if extension == "cpp" {
                    let cmakefile = gen_dir.join("CMakeLists.txt");
                    // currently, CMakeLists are same with generator
                    let cmakedata =
                        super::template::get_checker_cmakefile(super::template::CheckerOptions {});
                    tokio::fs::write(cmakefile, cmakedata)
                        .await
                        .context("write generator's CMakeLists.txt")?;
                }
            }
        }
        Ok(())
    }

    async fn process_checker(
        &mut self,
        node_checker: roxmltree::Node<'_, '_>,
    ) -> anyhow::Result<()> {
        self.tx.send_event(Update::ImportChecker).await?;
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

    async fn process_executable(
        &mut self,
        node_executable: roxmltree::Node<'_, '_>,
    ) -> anyhow::Result<()> {
        for node_source in node_executable.children() {
            if node_source.tag_name().name() != "source" {
                continue;
            }
            let attr_path = node_source
                .attribute("path")
                .context("<source> does not have path attribute")?;
            let attr_type = node_source
                .attribute("type")
                .context("<source> does not have type attribute")?;
            self.process_file(attr_path, attr_type).await?;
        }
        Ok(())
    }

    async fn process_tests(&mut self, tests_node: roxmltree::Node<'_, '_>) -> anyhow::Result<()> {
        self.tx.send_event(Update::ImportTests).await?;
        assert_eq!(tests_node.tag_name().name(), "tests");
        let mut cnt: usize = 0;
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
                group: format!(
                    "g{}",
                    test_node
                        .attribute("group")
                        .unwrap_or("default")
                        .to_string()
                ),
            };
            let is_generated = test_node.attribute("method").unwrap() == "generated";
            if is_generated {
                let cmd_iter = test_node.attribute("cmd").unwrap().split_whitespace();
                let testgen_cmd = cmd_iter.map(ToOwned::to_owned).collect::<Vec<_>>();
                let gen_name = testgen_cmd[0].clone();
                self.known_generators.insert(gen_name);
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
        self.tx
            .send_event(Update::ImportTestsDone { count: cnt })
            .await?;
        Ok(())
    }

    async fn process_solutions(&mut self, node: roxmltree::Node<'_, '_>) -> anyhow::Result<()> {
        self.tx.send_event(Update::ImportSolutions).await?;
        for solution_node in node.children() {
            if !solution_node.is_element() {
                continue;
            }
            let tag = solution_node
                .attribute("tag")
                .context("solution does not have <tag> attribute")?;
            if tag == "main" {
                self.tx
                    .send_event(Update::ImportSolution(tag.to_string()))
                    .await?;
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
                tokio::fs::create_dir_all(&dir)
                    .await
                    .context("create main solution dir")?;
                self.import_file(Path::new(&src_path), Path::new("solutions/main/main.cpp"))?;
                {
                    let cmake_path = dir.join("CMakeLists.txt");
                    let data = include_str!("./solution.cmake");
                    std::fs::write(&cmake_path, data)
                        .context("write CMakeLists.txt for solution")?;
                }
            } else {
                let message = format!(
                    "skipping solution with tag {}: importing non-main solutions not yet implemented",
                    tag
                );
                self.tx.send_event(Update::Warning(message)).await?;
            }
        }
        Ok(())
    }

    async fn process_names(&mut self, node_names: roxmltree::Node<'_, '_>) -> anyhow::Result<()> {
        assert!(node_names.is_element());
        for child in node_names.children() {
            if !child.is_element() {
                continue;
            }
            let title = child
                .attribute("value")
                .context("<name> does not have value attribute")?;

            self.problem_cfg.title = title.to_string();
            self.tx
                .send_event(Update::Property {
                    property_name: PropertyName::ProblemTitle,
                    property_value: title.to_string(),
                })
                .await?;
            break;
        }
        Ok(())
    }

    fn process_problem(&mut self, node_problem: roxmltree::Node) {
        assert_eq!(node_problem.tag_name().name(), "problem");
        if let Some(name) = node_problem.attribute("short-name") {
            self.problem_cfg.name = name.to_string();
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
        m.valuer_cfg = Some("valuer.yaml".to_string());
        let mut random_seed = [0; 8];
        getrandom::getrandom(&mut random_seed)?;
        let rand_seed_hex = hex::encode(&random_seed);
        assert_eq!(rand_seed_hex.len(), 16);
        m.random_seed = Some(rand_seed_hex);
        Ok(())
    }

    fn init_dirs(&mut self) -> anyhow::Result<()> {
        for suf in &[
            "solutions",
            "generators",
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

    fn go<'b>(
        &'b mut self,
        node: roxmltree::Node<'b, 'b>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'b>> {
        Box::pin(async move {
            for child in node.children() {
                self.feed(child).await?;
            }
            Ok(())
        })
    }

    async fn feed(&mut self, node: roxmltree::Node<'_, '_>) -> anyhow::Result<()> {
        match node.tag_name().name() {
            "names" => self.process_names(node).await?,
            "solutions" => self.process_solutions(node).await?,
            "judging" => self.process_judging_section(node).await?,
            "executable" => self.process_executable(node).await?,
            "checker" => self.process_checker(node).await?,
            "problem" => {
                self.process_problem(node);
                self.go(node).await?;
            }
            _ => {
                self.go(node).await?;
            }
        }
        Ok(())
    }

    async fn import_valuer_config(&mut self) -> anyhow::Result<()> {
        let valuer_cfg_path = self.src.join("files/valuer.cfg");
        let config = if valuer_cfg_path.exists() {
            self.tx.send_event(Update::ImportValuerConfig).await?;
            let (config, warnings) = super::valuer_cfg::import(&valuer_cfg_path).await?;
            for warn in warnings {
                self.tx
                    .send_event(Update::Warning(format!(
                        "while importing valuer config: {}",
                        warn
                    )))
                    .await?;
            }
            serde_yaml::to_string(&config)?
        } else {
            self.tx.send_event(Update::DefaultValuerConfig).await?;
            include_str!("./default_valuer_config.yaml").to_string()
        };
        tokio::fs::write(self.dest.join("valuer.yaml"), config).await?;
        Ok(())
    }

    pub(crate) async fn run(&mut self) -> anyhow::Result<()> {
        self.init_dirs()?;
        self.fill_manifest()?;
        self.feed(self.doc).await?;
        self.import_valuer_config().await?;
        Ok(())
    }
}
