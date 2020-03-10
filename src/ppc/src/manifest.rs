use anyhow::{bail, Context};
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
    #[serde(default)]
    pub limits: pom::Limits,
    pub group: String,
}

impl RawTestsSpec {
    fn parse_mapping_chunk(&self, ch: &str) -> anyhow::Result<Vec<u32>> {
        if ch.contains("..") {
            let parts: Vec<_> = ch.split("..").collect();
            if parts.len() != 2 {
                bail!("range map chunk must look like x..y");
            }
            let parts: Result<Vec<_>, _> = parts.into_iter().map(|x| x.parse::<u32>()).collect();
            match parts {
                Ok(parts) => {
                    let begin = parts[0];
                    let end = parts[1];
                    if begin > end {
                        bail!("range begin must be less than or equal to range end");
                    }
                    let idxs: Vec<_> = std::ops::RangeInclusive::new(begin, end).collect();
                    return Ok(idxs);
                }
                Err(e) => {
                    bail!("couldn't parse range bound: {}", e);
                }
            }
        }

        match ch.parse() {
            Ok(num) => Ok(vec![num]),
            Err(err) => bail!("couldn't parse number: {}", err),
        }
    }

    fn parse_mapping(&self) -> anyhow::Result<Vec<u32>> {
        let chunks = self.map.split(',');
        let mut out = vec![];
        for ch in chunks {
            match self.parse_mapping_chunk(ch) {
                Ok(idxs) => {
                    out.extend(idxs.into_iter());
                }
                Err(err) => bail!("failed to parse '{}': {:#}", ch, err),
            }
        }
        if !out.is_sorted() {
            bail!("mapping is not sorted");
        }
        Ok(out)
    }

    fn postprocess(&self) -> anyhow::Result<Vec<(u32, TestSpec)>> {
        {
            let mut cnt = 0;
            if self.files.is_some() {
                cnt += 1;
            }
            if self.testgen.is_some() {
                cnt += 1;
            }
            if cnt == 2 {
                bail!("exactly one of 'files' and 'testgen' must be specified");
            }
        }
        let idxs = self.parse_mapping()?;
        let mut out = Vec::new();
        if let Some(file_tpl) = &self.files {
            for &id in idxs.iter() {
                let res =
                    formatf::format(file_tpl.as_bytes(), &[formatf::Value::Int(i128::from(id))]);
                match res {
                    Ok(file) => {
                        let file =
                            String::from_utf8(file).expect("interpolation provided non-utf8 data");
                        out.push((id, TestGenSpec::File { path: file }));
                    }
                    Err(err) => {
                        bail!("formatting error: {:?}", err);
                        // TODO: implement Display for formatf FormatError
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
            .map(|(id, test_gen_spec)| {
                (
                    id,
                    TestSpec {
                        gen: test_gen_spec,
                        limits: self.limits,
                        group: self.group.clone(),
                    },
                )
            })
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
    pub limits: pom::Limits,
    pub group: String,
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

    #[serde(rename = "valuer-cfg")]
    pub valuer_cfg: Option<String>,

    #[serde(default)]
    pub limits: pom::Limits,
}

impl RawProblem {
    fn process_tests(&self) -> anyhow::Result<Vec<TestSpec>> {
        let mut tests = Vec::new();
        for test_spec in &self.tests {
            let mut new_tests = test_spec
                .postprocess()
                .context("bad test description block")?;

            tests.append(&mut new_tests);
        }
        tests.sort_by_key(|item| item.0);
        let test_ids: Vec<_> = tests.iter().map(|item| item.0).collect();
        if test_ids.is_empty() {
            bail!("No tests specified");
        }

        for i in 1..test_ids.len() {
            if test_ids[i - 1] == test_ids[i] {
                bail!("test {} is specified more than once", test_ids[i]);
            }
        }
        for (i, tid) in test_ids.iter().enumerate() {
            if i + 1 != *tid as usize {
                bail!("test {} is not specified", i + 1);
            }
        }
        Ok(tests.into_iter().map(|item| item.1).collect())
    }

    pub fn postprocess(mut self) -> anyhow::Result<(Problem, /* warnings */ Vec<String>)> {
        let mut warnings = Vec::new();
        let tests = self.process_tests()?;

        let random_seed = match self.random_seed.take() {
            Some(s) => {
                if s.len() != 16 {
                    bail!("random-seed must have length16");
                }
                if s.chars().all(|c| c.is_ascii_hexdigit()) {
                    s.to_lowercase()
                } else {
                    bail!("random-seed is not hex");
                }
            }
            None => {
                warnings.push("random-seed not present, hardcoded seed is used".to_string());
                "6a2c1750a93b8184".to_string()
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
                            bail!("check-type=custom requires [custom-check] section");
                        }
                    };
                    Check::Custom(custom_check)
                }
                "builtin" => {
                    let builtin_check = match self.builtin_check {
                        Some(bc) => bc,
                        None => {
                            bail!("check-type=builtin requires [builtin-check] section");
                        }
                    };
                    Check::Builtin(builtin_check)
                }
                other => {
                    bail!("unknown check type: {}", other);
                }
            },
            tests,
            name: self.name,
            random_seed,
            check_options: self.check_options.unwrap_or_else(|| CheckOptions {
                args: vec![], // do not pass additional argv to checker it they are not provided
            }),
            valuer: self.valuer,
            valuer_cfg: self.valuer_cfg,
            limits: self.limits,
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
    pub valuer_cfg: Option<String>,
    pub limits: pom::Limits,
}
