use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomCheck {
    #[serde(rename = "pass-correct")]
    pub pass_correct: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuiltinCheck {
    #[serde(rename = "name")]
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawTestsSpec {
    map: String,
    testgen: Option<String>,
    files: Option<String>,
}

impl RawTestsSpec {
    fn parse_mapping_chunk(&self, ch: &str) -> Result<Vec<u32>, String> {
        if ch.contains("..") {
            let parts: Vec<_> = ch.split("..").collect();
            if parts.len() != 2 {
                return Err("range map chunk must look like x..y".to_string());
            }
            let parts: Result<Vec<_>, _> = parts.into_iter().map(|x| x.parse::<u32>()).collect();
            match parts {
                Ok(parts) => {
                    let begin = parts[0];
                    let end = parts[1];
                    if begin > end {
                        return Err(
                            "range begin must be less than or equal to range end".to_string()
                        );
                    }
                    let idxs: Vec<_> = std::ops::RangeInclusive::new(begin, end).collect();
                    return Ok(idxs);
                }
                Err(e) => {
                    return Err(format!("couldn't parse range bound: {}", e.to_string()));
                }
            }
        }

        match ch.parse() {
            Ok(num) => Ok(vec![num]),
            Err(err) => Err(format!("couldn't parse number: {}", err.to_string())),
        }
    }

    fn parse_mapping(&self) -> Result<Vec<u32>, String> {
        let chunks = self.map.split(',');
        let mut out = vec![];
        for ch in chunks {
            match self.parse_mapping_chunk(ch) {
                Ok(idxs) => {
                    out.extend(idxs.into_iter());
                }
                err => {
                    return err;
                }
            }
        }
        if !out.is_sorted() {
            return Err("mapping is not sorted".to_string());
        }
        Ok(out)
    }

    fn postprocess(&self) -> Result<Vec<(u32, TestSpec)>, String> {
        if self.files.as_ref().xor(self.testgen.as_ref()).is_none() {
            return Err("exactly one of 'files' and 'testgen' must be specified".to_string());
        }
        let idxs = self.parse_mapping()?;
        let mut out = Vec::new();
        if let Some(file_tpl) = &self.files {
            for &id in idxs.iter() {
                let res = rt_format!(file_tpl, id).map_err(|err| err.to_string());
                match res {
                    Ok(file) => {
                        out.push((id, TestGenSpec::File { path: file }));
                    }
                    Err(err) => {
                        return Err(format!("formatting error: {}", err));
                    }
                }
            }
        }
        if let Some(testgen_name) = &self.testgen {
            let spec = TestGenSpec::Generate {
                testgen: testgen_name.clone(),
            };

            for &id in &idxs {
                out.push((id, spec.clone()));
            }
        }
        let out = out
            .into_iter()
            .map(|(id, test_gen_spec)| (id, TestSpec { gen: test_gen_spec }))
            .collect();

        Ok(out)
    }
}

#[derive(Clone, Debug)]
pub enum TestGenSpec {
    Generate { testgen: String },
    File { path: String },
}

#[derive(Debug)]
pub struct TestSpec {
    pub gen: TestGenSpec,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawProblem {
    #[serde(rename = "primary-solution")]
    primary_solution: String,
    #[serde(rename = "check-type")]
    check_type: String,
    #[serde(rename = "custom-check")]
    custom_check: Option<CustomCheck>,
    #[serde(rename = "builtin-check")]
    builtin_check: Option<BuiltinCheck>,
    tests: Vec<RawTestsSpec>,
}

impl RawProblem {
    pub fn postprocess(self) -> Result<Problem, String> {
        let mut tests = Vec::new();
        for test_spec in self.tests {
            let res = test_spec.postprocess();
            match res {
                Ok(mut new_tests) => {
                    tests.append(&mut new_tests);
                }
                Err(description) => {
                    return Err(format!(
                        "couldn't process test description block: {}",
                        description
                    ));
                }
            }
        }
        tests.sort_by_key(|item| item.0);
        let test_ids: Vec<_> = tests.iter().map(|item| item.0).collect();
        if test_ids.is_empty() {
            return Err("No tests specified".to_owned());
        }

        for i in 1..test_ids.len() {
            if test_ids[i - 1] == test_ids[i] {
                return Err(format!("test {} is specified more than once", test_ids[i]));
            }
        }
        for (i, tid) in test_ids.iter().enumerate() {
            if i + 1 != *tid as usize {
                return Err(format!("test {} is not specified", i + 1));
            }
        }
        let tests: Vec<_> = tests.into_iter().map(|item| item.1).collect();

        let out = Problem {
            primary_solution: self.primary_solution,
            check: match self.check_type.as_str() {
                "custom" => {
                    let custom_check = match self.custom_check {
                        Some(cs) => cs,
                        None => {
                            return Err(
                                "check-type=custom specified, but [custom-check] section is absent"
                                    .to_owned(),
                            );
                        }
                    };
                    Check::Custom(custom_check)
                }
                "builtin" => {
                    let builtin_check = match self.builtin_check {
                        Some(bc) => bc,
                        None => {
                            return Err(
                                "check-type=builtin specified, but [builtin-check] section is absent"
                                    .to_owned()
                            );
                        }
                    };
                    Check::Builtin(builtin_check)
                }
                other => {
                    return Err(format!("unknown check type: {}", other));
                }
            },
            tests,
        };

        Ok(out)
    }
}

#[derive(Debug)]
pub enum Check {
    Custom(CustomCheck),
    Builtin(BuiltinCheck),
}

#[derive(Debug)]
pub struct Problem {
    pub primary_solution: String,
    pub check: Check,
    pub tests: Vec<TestSpec>,
}
