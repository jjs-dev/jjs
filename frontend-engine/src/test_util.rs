/// Utilities for testing
#[derive(Debug, Clone)]
pub struct Response(pub serde_json::Value);

impl std::ops::Deref for Response {
    type Target = serde_json::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Response {
    pub fn is_ok(&self) -> bool {
        self.0.get("errors").is_none()
    }

    pub fn unwrap_ok(self) -> serde_json::Value {
        if self.is_ok() {
            self.0.get("data").unwrap().clone()
        } else {
            let errs = self
                .0
                .get("errors")
                .expect("errors missing on failed request")
                .as_array()
                .expect("errors field must be array");
            assert!(!errs.is_empty());
            eprintln!("Error: query failed");
            eprintln!("Server response contains errors:");
            for (i, err) in errs.iter().enumerate() {
                if i != 0 {
                    eprintln!("------");
                }
                eprintln!("{}", ErrorPrettyPrinter(&err));
            }
            panic!("Operation failed unexpectedly");
        }
    }

    pub fn unwrap_errs(self) -> Vec<serde_json::Value> {
        if self.is_ok() {
            eprintln!("Error: query with fail=true succeeded");
            eprintln!("Response: \n{:?}", self.0);
            panic!("Operation succeeded unexpectedly");
        } else {
            let errs = self.0.get("errors").unwrap().as_array().unwrap();
            assert!(!errs.is_empty());
            errs.clone()
        }
    }
}

pub struct ErrorPrettyPrinter<'a>(pub &'a serde_json::Value);

impl std::fmt::Display for ErrorPrettyPrinter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut err = self.0.as_object().unwrap().clone();
        let ext = err.remove("extensions");
        writeln!(f, "{}", serde_json::to_string_pretty(&err).unwrap())?;
        if let Some(ext) = ext {
            if let Some(ext) = ext.as_object() {
                writeln!(f, "extensions:\n")?;
                if let Some(error_code) = ext.get("errorCode") {
                    writeln!(f, "error code: {}", error_code.to_string())?;
                }
                if let Some(backtrace) = ext.get("trace") {
                    writeln!(f, "backtrace: {}", backtrace.as_str().unwrap())?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct RequestBuilder {
    vars: Option<serde_json::Value>,
    operation: Option<String>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn vars(&mut self, v: &serde_json::Value) -> &mut Self {
        assert!(v.is_object());
        self.vars = Some(v.clone());
        self
    }

    pub fn operation(&mut self, op: &str) -> &mut Self {
        self.operation = Some(op.to_string());
        self
    }

    pub fn to_query(&self) -> String {
        let obj = serde_json::json!({
             "query": self.operation.as_ref().unwrap(),
             "variables": self.vars.clone().unwrap_or_else(||serde_json::Value::Null),
        });
        serde_json::to_string(&obj).unwrap()
    }
}

pub fn check_error(err: &serde_json::Value, exp_code: &str) {
    let code = err
        .get("extensions")
        .and_then(|v| v.get("errorCode"))
        .and_then(|v| v.as_str())
        .map(|x| x.to_string());
    assert_eq!(code.as_ref().map(String::as_str), Some(exp_code));
}
