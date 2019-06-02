use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomCheck {
    #[serde(rename = "pass-correct")]
    pass_correct: bool,
    #[serde(rename = "protocol-version")]
    proto_version: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawProblem {
    #[serde(rename = "primary-solution")]
    primary_solution: String,
    #[serde(rename = "check-type")]
    check_type: String,
    #[serde(rename = "custom-check")]
    custom_check: Option<CustomCheck>,
}

impl RawProblem {
    pub fn postprocess(self) -> Result<Problem, String> {
        let out = Problem {
            primary_solution: self.primary_solution,
            check: match self.check_type.as_str() {
                "custom" => {
                    let custom_check =
                        match self.custom_check {
                            Some(cs) => cs,
                            None => return Err(format!(
                                "check-type=custom specified, but [custom-check] section is absent"
                            )),
                        };
                    Check::Custom(custom_check)
                }
                other => {
                    return Err(format!("unknown check type: {}", other));
                }
            },
        };

        Ok(out)
    }
}

#[derive(Debug)]
pub enum Check {
    Custom(CustomCheck),
}

#[derive(Debug)]
pub struct Problem {
    primary_solution: String,
    check: Check,
}
