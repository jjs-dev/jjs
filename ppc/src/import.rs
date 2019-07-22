use std::{
    collections::HashMap,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};
use xml::reader::XmlEvent;

struct Importer<'a> {
    src: &'a Path,
    dest: &'a Path,
    problem_cfg: crate::cfg::RawProblem,
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

    fn read_tag_content_as_string(&self, iter: &mut impl Iterator<Item=XmlEvent>) -> String {
        let event = iter.next().unwrap();
        let data = self.event_as_string(event);
        let close_event = iter.next().unwrap();
        match &close_event {
            XmlEvent::EndElement { .. } => data,
            _ => panic!("expected end of tag, got {:?}", close_event),
        }
    }

    fn skip_section(&self, manifest: &mut impl Iterator<Item=XmlEvent>) {
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
    fn process_judging_section(&self, manifest: &mut impl Iterator<Item=XmlEvent>) {
        let testset_start = manifest.next().unwrap();
        match testset_start {
            XmlEvent::StartElement { name, .. } => {
                assert_eq!(name.local_name, "testset");
            }
            _ => {
                panic!("unexpected event: {:?}", testset_start);
            }
        }
        println!("parsing tests info");
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
                    _ => {
                        eprintln!(
                            "warning: unexpected tag in <problem><judging><testset>: {}",
                            name.local_name
                        );
                        self.skip_section(manifest);
                    }
                },
                XmlEvent::EndElement { .. } => break,
                _ => continue,
            }
        }
    }

    fn process_problem(&mut self, manifest: &mut impl Iterator<Item=XmlEvent>) {
        loop {
            match manifest.next().unwrap() {
                XmlEvent::StartElement { name, .. } => {
                    if name.local_name == "judging" {
                        self.process_judging_section(manifest);
                    } else if name.local_name == "names" {
                        self.process_names(manifest);
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
    }

    fn process_names(&mut self, iter: &mut impl Iterator<Item=XmlEvent>) {
        let ev = iter.next().unwrap();
        match ev {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                assert_eq!(name.local_name, "name");
                let attrs = self.parse_attrs(attributes);
                let title = attrs["value"].to_string();
                self.problem_cfg.title = title;
                self.skip_section(iter);
            }
            _ => {
                panic!("parsing <names>: unexpected event {:?}", ev);
            }
        }
        self.skip_section(iter);
    }

    fn fill_manifest(&mut self) {
        let m = &mut self.problem_cfg;
        m.valuer = "icpc".to_string();
        m.check_type = "custom".to_string()
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
        self.fill_manifest();
        loop {
            let event = event_iter.next().unwrap();
            match event {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    assert_eq!(name.local_name, "problem");
                    let attrs = self.parse_attrs(attributes);
                    self.problem_cfg.name = attrs
                        .get("short-name")
                        .map(|x| x.to_string())
                        .expect("missing short-name in <problem>");
                    self.process_problem(&mut event_iter);
                    return;
                }
                _ => {}
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
    };

    importer.run();

    let manifest_path = dest.join("problem.toml");
    let manifest_data =
        toml::ser::to_string_pretty(&importer.problem_cfg).expect("serialize ppc config");
    std::fs::write(manifest_path, manifest_data).expect("write ppc manifest");
}
