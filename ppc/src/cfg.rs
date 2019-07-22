use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CustomCheck {
    #[serde(rename = "pass-correct")]
    pub pass_correct: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct BuiltinCheck {
    #[serde(rename = "name")]
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CheckOptions {
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RawTestsSpec {
    pub map: String,
    pub testgen: Option<Vec<String>>,
    pub files: Option<String>,
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
        {
            let mut cnt = 0;
            if self.files.is_some() {
                cnt += 1;
            }
            if self.testgen.is_some() {
                cnt += 1;
            }
            if cnt == 2 {
                return Err("exactly one of 'files' and 'testgen' must be specified".to_string());
            }
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
        if let Some(testgen_cmd) = &self.testgen {
            let spec = TestGenSpec::Generate {
                testgen: testgen_cmd[0].clone(),
                args: testgen_cmd[1..].to_vec(),
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
    Generate { testgen: String, args: Vec<String> },
    File { path: String },
}

#[derive(Debug)]
pub struct TestSpec {
    pub gen: TestGenSpec,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RawProblem {
    pub title: String,

    pub name: String,

    #[serde(rename = "random-seed")]
    pub random_seed: Option<String>,

    #[serde(rename = "primary-solution")]
    pub primary_solution: Option<String>,

    #[serde(rename = "check-type")]
    pub check_type: String,

    pub valuer: String,

    #[serde(rename = "custom-check")]
    pub custom_check: Option<CustomCheck>,

    #[serde(rename = "builtin-check")]
    pub builtin_check: Option<BuiltinCheck>,

    pub tests: Vec<RawTestsSpec>,

    #[serde(rename = "check-options")]
    pub check_options: Option<CheckOptions>,
}

impl RawProblem {
    fn process_tests(&self) -> Result<Vec<TestSpec>, String> {
        let mut tests = Vec::new();
        for test_spec in &self.tests {
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
        Ok(tests.into_iter().map(|item| item.1).collect())
    }

    pub fn postprocess(mut self) -> Result<(Problem, /* warnings */ Vec<String>), String> {
        let mut warnings = Vec::new();
        let tests = self.process_tests()?;

        let random_seed = match self.random_seed.take() {
            Some(s) => {
                if s.len() != 64 {
                    return Err("random-seed must have length of 64".to_string());
                }
                if s.chars().all(|c| c.is_ascii_hexdigit()) {
                    s.to_lowercase()
                } else {
                    return Err("random-seed is not hex".to_string());
                }
            }
            None => {
                warnings.push("random-seed not present, hardcoded seed is used".to_string());
                "1f56fd326365e6184b133b04e330b456004a1c852f2a9cf26a2c1750a93b8184".to_string()
            }
        };

        let out = Problem {
            title: self.title,
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
            name: self.name,
            random_seed,
            check_options: self.check_options.unwrap_or_else(|| CheckOptions {
                args: vec![], // do not pass additional argv to checker
            }),
            valuer: self.valuer,
        };

        Ok((out, warnings))
    }
}

#[derive(Debug)]
pub enum Check {
    Custom(CustomCheck),
    Builtin(BuiltinCheck),
}

#[derive(Debug)]
pub struct Problem {
    pub title: String,
    pub name: String,
    pub primary_solution: Option<String>,
    pub check: Check,
    pub tests: Vec<TestSpec>,
    pub random_seed: String,
    pub check_options: CheckOptions,
    pub valuer: String,
}
