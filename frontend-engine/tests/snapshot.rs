use std::{env, fs, io::BufReader};

#[derive(Clone, Copy, Eq, PartialEq)]
enum Outcome {
    Success,
    Error,
}

struct Step {
    query: String,
    variables: serde_json::Value,
    response: serde_json::Value,
    outcome: Outcome,
}

impl Step {
    fn parse(v: &serde_yaml::Value) -> Step {
        let query = v.get("query").unwrap().as_str().unwrap().to_string();
        let variables = v
            .get("vars")
            .map(Clone::clone)
            .unwrap_or(serde_yaml::Value::Null);
        let variables = serde_yaml::from_value(variables).unwrap();
        let response = v.get("res").map(Clone::clone).unwrap();
        let response = serde_yaml::from_value(response).unwrap();

        let mut outcome = Outcome::Success;

        if let Some(oc) = v.get("fail") {
            let oc = oc.as_bool().unwrap();
            if oc {
                outcome = Outcome::Error;
            }
        }

        Step {
            query,
            variables,
            response,
            outcome,
        }
    }
}

struct TestCase {
    name: String,
    steps: Vec<Step>,
}

impl TestCase {
    fn parse(v: &serde_yaml::Value) -> TestCase {
        let steps = v.get("steps").unwrap().as_sequence().unwrap();
        let steps = steps.iter().map(|v| Step::parse(v)).collect::<Vec<_>>();
        TestCase {
            name: "".to_string(),
            steps,
        }
    }
}

fn compare_responses(expected: &serde_json::Value, actual: &serde_json::Value) {
    assert_eq!(expected, actual);
}

fn check_snapshot(test_case: TestCase) -> bool {
    println!("running snapshot {}", &test_case.name);
    let server = frontend_engine::ApiServer::create_embedded();
    let client = rocket::local::Client::new(server).unwrap();
    for step in test_case.steps {
        let obj = serde_json::json!({
             "query": step.query,
             "variables": step.variables,
        });
        let body = serde_json::to_string(&obj).unwrap();
        let mut req = client
            .post("/graphql")
            .body(body)
            .header(rocket::http::ContentType::JSON)
            .dispatch();
        assert_eq!(req.status(), rocket::http::Status::Ok);
        assert_eq!(
            req.content_type(),
            Some("application/json".parse().unwrap())
        );
        let body = req.body_string().unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        let body = body.as_object().unwrap();
        let data = body.get("data");
        let errors = body.get("errors");
        match errors {
            Some(errs) => {
                let errs = errs.as_array().unwrap();
                assert!(!errs.is_empty());
                if step.outcome == Outcome::Success {
                    eprintln!("Error: query failed");
                    eprintln!("Server response contains errors:");
                    for (i, err) in errs.iter().enumerate() {
                        if i != 0 {
                            eprintln!("------");
                        }
                        eprintln!("{}", serde_json::to_string(err).unwrap());
                    }
                    return false;
                }
            }
            None => {
                if step.outcome == Outcome::Error {
                    eprintln!("Error: query with fail=true succeeded");
                    return false;
                }
                compare_responses(&step.response, data.unwrap());
            }
        }
    }
    true
}

#[test]
fn main() {
    println!("running snapshot-based tests");
    let snapshots_dir = env::current_dir().unwrap().join("tests/snapshots");
    let items = fs::read_dir(&snapshots_dir).unwrap();
    for item in items {
        let item = item.unwrap();
        let path = item.path();
        let test_case_data = fs::File::open(&path).unwrap();
        let test_case_data = BufReader::new(test_case_data);
        let test_case_data = serde_yaml::from_reader(test_case_data).unwrap();
        let mut test_case = TestCase::parse(&test_case_data);
        test_case.name = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .split('.')
            .next()
            .unwrap()
            .to_string();
        check_snapshot(test_case);
    }
}
