//! Checker out file parser
use snafu::Snafu;
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

#[derive(Snafu, Debug)]
pub enum Error {
    UnknownTag { line: u32, tag: String },
    ParseError { line: u32, description: String },
    TagMissing { tag: String },
    TagRedefined { tag: String },
    TagFormat { tag: String, error_message: String },
}

pub fn parse(data: &str) -> Result<Output, Error> {
    let mut res_outcome = None;
    for (line_id, line) in data.lines().enumerate() {
        let line_id = line_id as u32;
        let p = match line.find('=') {
            Some(i) => i,
            None => {
                return ParseError {
                    line: line_id,
                    description: "Line doesn't contain '='-separated key and value".to_string(),
                }
                .fail();
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
                        let msg = e.to_string();
                        return TagFormat {
                            tag: tag.to_string(),
                            error_message: msg,
                        }
                        .fail();
                    }
                };
                if res_outcome.replace(outcome).is_some() {
                    return TagRedefined {
                        tag: tag.to_string(),
                    }
                    .fail();
                }
            }
            _ => {
                return UnknownTag {
                    line: line_id,
                    tag: tag.to_string(),
                }
                .fail();
            }
        }
    }
    let outcome = match res_outcome {
        Some(o) => o,
        None => {
            return TagMissing {
                tag: "outcome".to_string(),
            }
            .fail();
        }
    };
    Ok(Output { outcome })
}
