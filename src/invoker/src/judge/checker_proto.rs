//! Checker out file parser
use anyhow::bail;
use strum_macros::EnumString;

#[derive(EnumString)]
pub enum Outcome {
    Ok,
    WrongAnswer,
    PresentationError,
    #[strum(to_string = "CheckerLogicError")]
    BadChecker,
}

pub struct Output {
    pub outcome: Outcome,
}

pub fn parse(data: &str) -> anyhow::Result<Output> {
    let mut res_outcome = None;
    for (line_id, line) in data.lines().enumerate() {
        let line_id = (line_id + 1) as u32;
        let p = match line.find('=') {
            Some(i) => i,
            None => {
                bail!(
                    "Line {} doesn't contain '='-separated key and value",
                    line_id
                );
            }
        };
        let tag = &data[..p];
        let value = &data[p + 1..];
        match tag {
            "outcome" => {
                let data = value.trim();
                let outcome: Outcome = match data.parse() {
                    Ok(o) => o,
                    Err(e) => {
                        bail!("Tag outcome: {}", e);
                    }
                };
                if res_outcome.replace(outcome).is_some() {
                    bail!("Tag outcome redefined");
                }
            }
            _ => {
                bail!("Line {}: unknown tag {}", line_id, tag);
            }
        }
    }
    let outcome = match res_outcome {
        Some(o) => o,
        None => {
            bail!("Tag outcome missong");
        }
    };
    Ok(Output { outcome })
}
