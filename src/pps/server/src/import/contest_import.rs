// TODO revive when contests are reintroduced
use std::{borrow::Cow, path::Path};
use thiserror::Error;
#[derive(Error, Debug)]
pub(super) enum ImportContestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("syntax error: {0}")]
    XmlSyn(#[from] roxmltree::Error),
    #[error("bad context.xml: {0}")]
    BadManifest(Cow<'static, str>),
}

fn go(node: roxmltree::Node, cfg: &mut entity::Contest) -> Result<(), ImportContestError> {
    match node.tag_name().name() {
        "problem" => {
            let index = node.attribute("index").ok_or_else(|| {
                ImportContestError::BadManifest("index attribute missing in <problem />".into())
            })?;
            let url = node.attribute("url").ok_or_else(|| {
                ImportContestError::BadManifest("url attribute missing in <problem />".into())
            })?;
            let problem_name = url
                .rsplit('/')
                .next()
                .expect("rsplit() should never be empty");
            let binding = entity::entities::contest::ProblemBinding {
                name: problem_name.to_string(),
                code: index.to_string(),
            };
            cfg.problems.push(binding);
            Ok(())
        }
        _ => {
            for child in node.children() {
                go(child, cfg)?;
            }
            Ok(())
        }
    }
}

pub(super) fn import(
    path: &Path,
    contest_name: &str,
) -> Result<entity::Contest, ImportContestError> {
    let data = std::fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&data)?;
    let mut cfg = entity::Contest {
        title: "".to_string(),
        id: contest_name.to_string(),
        problems: vec![],
        judges: vec![],
        group: vec![],
        unregistered_visible: false,
        anon_visible: false,
        duration: None,
        end_time: None,
        start_time: None,
        is_virtual: false,
    };
    go(doc.root(), &mut cfg)?;
    Ok(cfg)
}
