#[derive(Clone, Copy, Eq, PartialEq)]
enum Outcome {
    Success,
    Error,
}

pub struct Step {
    query: String,
    variables: serde_json::Value,
    response: serde_json::Value,
    outcome: Outcome,
}

fn compare_responses(expected: &serde_json::Value, actual: &serde_json::Value) {
    assert_eq!(expected, actual);
}

fn print_error(err: &serde_json::Value) {
    let mut err = err.as_object().unwrap().clone();
    let ext = err.remove("extensions");
    println!("{}", serde_json::to_string_pretty(&err).unwrap());
    if let Some(ext) = ext {
        if let Some(ext) = ext.as_object()
        {
            println!("extensions:\n");
            if let Some(error_code) = ext.get("errorCode") {
                println!("error code: {}", error_code.to_string());
            }
            if let Some(backtrace) = ext.get("trace") {
                println!("backtrace: {}", backtrace.as_str().unwrap());
            }
        }
    }
}

fn eval_function(fn_name: &str, data: &serde_yaml::Value) -> serde_json::Value {
    match fn_name {
        "base64::encode" => {
            let data = data.as_str().unwrap();
            let data = base64::encode(data);
            serde_json::Value::String(data)
        }
        _ => panic!("unknown function {}", fn_name)
    }
}

impl Step {
    pub fn parse(v: &serde_yaml::Value) -> Step {
        let query = v.get("query").unwrap().as_str().unwrap().to_string();
        let variables = v
            .get("vars")
            .map(Clone::clone)
            .unwrap_or(serde_yaml::Value::Null);
        let mut variables: serde_json::Value = serde_yaml::from_value(variables).unwrap();
        if variables.is_null() {
            variables = serde_json::Value::Object(Default::default());
        }
        assert!(variables.is_object());
        let response = v.get("res").map(Clone::clone).unwrap();
        let response = serde_yaml::from_value(response).unwrap();

        let mut outcome = Outcome::Success;

        if let Some(oc) = v.get("fail") {
            let oc = oc.as_bool().unwrap();
            if oc {
                outcome = Outcome::Error;
            }
        }

        if let Some(eval) = v.get("eval") {
            let eval = eval.as_sequence().unwrap();
            for item in eval {
                let var_name = item.get("var").unwrap().as_str().unwrap();
                let fn_name = item.get("fn").unwrap().as_str().unwrap();
                let arg = item.get("data").cloned().unwrap_or(serde_yaml::Value::Null);
                let val = eval_function(fn_name, &arg);
                variables.as_object_mut().unwrap().insert(var_name.to_string(), val);
            }
        }

        Step {
            query,
            variables,
            response,
            outcome,
        }
    }

    pub fn run(&self, client: &rocket::local::Client) -> bool {
        let obj = serde_json::json!({
             "query": self.query,
             "variables": self.variables,
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
                if self.outcome == Outcome::Success {
                    eprintln!("Error: query failed");
                    eprintln!("Server response contains errors:");
                    for (i, err) in errs.iter().enumerate() {
                        if i != 0 {
                            eprintln!("------");
                        }
                        print_error(&err);
                    }
                    return false;
                }
            }
            None => {
                if self.outcome == Outcome::Error {
                    eprintln!("Error: query with fail=true succeeded");
                    return false;
                }
                compare_responses(&self.response, data.unwrap());
            }
        }
        true
    }
}