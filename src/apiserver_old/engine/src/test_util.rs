/// Utilities for testing
#[derive(Debug, Clone)]
pub struct Response(pub serde_json::Value);

#[derive(Debug, PartialEq, Eq)]
pub enum Method {
    Delete,
    Patch,
}

impl std::ops::Deref for Response {
    type Target = serde_json::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Response {
    pub fn is_ok(&self) -> bool {
        self.0.get("error").is_none()
    }

    pub fn unwrap_ok(self) -> serde_json::Value {
        if self.is_ok() {
            self.0
        } else {
            let err = self.0.get("message").expect("message missing on response");
            eprintln!(
                "Error: query failed with error {}",
                err.as_str().expect("'message' field is not string")
            );
            eprintln!(
                "Server response contains error: {}",
                ErrorPrettyPrinter(&self.0["detail"])
            );
            panic!("Operation failed unexpectedly");
        }
    }

    pub fn unwrap_err(self) -> serde_json::Value {
        if self.is_ok() {
            eprintln!("Error: query with fail=true succeeded");
            eprintln!("Response: \n{:?}", self.0);
            panic!("Operation succeeded unexpectedly");
        } else {
            self.0
        }
    }
}

pub struct ErrorPrettyPrinter<'a>(pub &'a serde_json::Value);

impl std::fmt::Display for ErrorPrettyPrinter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ext = self.0.clone();
        writeln!(f, "{}", serde_json::to_string_pretty(&self.0).unwrap())?;

        if let Some(ext) = ext.as_object() {
            writeln!(f, "extensions:\n")?;
            if let Some(error_code) = ext.get("errorCode") {
                writeln!(f, "error code: {}", error_code.to_string())?;
            }
            if let Some(backtrace) = ext.get("trace") {
                writeln!(
                    f,
                    "backtrace: {}",
                    serde_json::to_string_pretty(&backtrace).unwrap()
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RequestBuilder {
    pub body: std::collections::HashMap<String, serde_json::Value>,
    pub action: Option<String>,
    pub method: Option<Method>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn var(&mut self, name: &str, val: &serde_json::Value) -> &mut Self {
        self.body.insert(name.to_string(), val.clone());
        self
    }

    pub fn action(&mut self, act: &str) -> &mut Self {
        self.action = Some(act.to_string());
        self
    }

    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = Some(method);
        self
    }
}

pub fn check_error(err: &serde_json::Value, exp_code: &str) {
    let code = err["detail"]["errorCode"].as_str().map(|x| x.to_string());
    if code.as_deref() == Some(exp_code) {
        return;
    }
    match code {
        Some(actual_code) => panic!(
            "Error code mismatch: expected `{}`, actual `{}`",
            exp_code, actual_code
        ),
        None => panic!("Expected `{}` error, got {:?}", exp_code, err),
    }
}
