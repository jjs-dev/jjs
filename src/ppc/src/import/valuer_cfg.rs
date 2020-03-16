use pest::Parser as _;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum ImportValuerCfgError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("valuer.cfg has syntax error: {0}")]
    Syntax(#[from] pest::error::Error<Rule>),
}

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "import/valuer_cfg.pest"]
pub(super) struct P;

struct Visitor<'a> {
    config: &'a mut svaluer::Config,
    tests_info: std::collections::HashMap<u32, String>,
}

impl<'a> Visitor<'a> {
    fn warn_not_sup(&self, feat: &str) {
        eprintln!("not supported feature: {}", feat);
    }

    fn visit_global_def(&mut self, _node: pest::iterators::Pair<'a, Rule>) {
        // all global options are not supported currently
        self.warn_not_sup("GlobalDefinitions");
    }

    fn visit_group_def(&mut self, node: pest::iterators::Pair<'a, Rule>) {
        assert_eq!(node.as_rule(), Rule::group_def);
        let mut iter = node.into_inner();
        let group_num_node = iter.next().unwrap();
        assert_eq!(group_num_node.as_rule(), Rule::num);
        let num: u32 = dbg!(group_num_node.as_str()).parse().unwrap();
        let mut group_cfg = svaluer::cfg::Group {
            name: format!("g{}", num),
            feedback: svaluer::cfg::FeedbackKind::Brief,
            tests_tag: None,
            run_to_first_failure: false,
            score: 0,
            deps: vec![],
        };
        for group_option in iter {
            self.visit_group_option(group_option, &mut group_cfg);
        }
        self.config.groups.push(group_cfg);
    }

    fn visit_group_option(
        &mut self,
        node: pest::iterators::Pair<'a, Rule>,
        group: &mut svaluer::cfg::Group,
    ) {
        match node.as_rule() {
            Rule::group_option_tests => {
                let mut iter = node.into_inner();
                let num1: u32 = iter.next().unwrap().as_str().parse().unwrap();
                let num2: u32 = iter.next().unwrap().as_str().parse().unwrap();
                assert!(num1 <= num2);
                for tid in num1..=num2 {
                    if self.tests_info.insert(tid, group.name.clone()).is_some() {
                        eprintln!("test {} is mentioned more than once", tid);
                    }
                }
            }
            Rule::group_option_score => {
                let sc = node.into_inner().next().unwrap().as_str().parse().unwrap();
                group.score = sc;
            }
            Rule::group_option_requires => {
                for num_node in node.into_inner() {
                    assert_eq!(num_node.as_rule(), Rule::num);
                    let group_id: u32 = num_node.as_str().parse().unwrap();
                    let dep_group_name = format!("g{}", group_id);
                    group
                        .deps
                        .push(svaluer::cfg::GroupRef::ByName(dep_group_name));
                }
            }
            other => panic!("{:?}", other),
        }
    }

    fn visit(&mut self, node: pest::iterators::Pair<'a, Rule>) {
        match node.as_rule() {
            Rule::config => {
                for child in node.into_inner() {
                    match child.as_rule() {
                        Rule::EOI => (),
                        Rule::definition => self.visit(child),
                        other => panic!("{:?}", other),
                    }
                }
            }
            Rule::definition => {
                let child = node.into_inner().next().unwrap();
                match child.as_rule() {
                    Rule::global_def => self.visit_global_def(child),
                    Rule::group_def => {
                        self.visit_group_def(child);
                    }
                    other => panic!("{:?}", other),
                }
            }
            _ => panic!("{:?}", node),
        }
    }
}

pub(crate) fn import(path: &Path) -> Result<svaluer::Config, ImportValuerCfgError> {
    let input = std::fs::read_to_string(path)?;
    let mut ast = P::parse(Rule::config, &input)?;
    let mut config = svaluer::Config { groups: Vec::new() };
    let mut visitor = Visitor {
        config: &mut config,
        tests_info: std::collections::HashMap::new(),
    };
    visitor.visit(ast.next().unwrap());
    Ok(config)
}
