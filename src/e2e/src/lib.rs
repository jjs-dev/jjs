pub struct RequestBuilder {
    builder: apiserver_engine::test_util::RequestBuilder,
    auth_token: Option<String>,
    client: reqwest::blocking::Client,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            builder: apiserver_engine::test_util::RequestBuilder::new(),
            auth_token: None,
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn var(&mut self, name: &str, val: impl Into<serde_json::Value>) -> &mut Self {
        self.builder.var(name, &val.into());
        self
    }

    pub fn action(&mut self, op: &str) -> &mut Self {
        self.builder.action(op);
        self
    }

    pub fn method(&mut self, method: apiserver_engine::test_util::Method) -> &mut Self {
        self.builder.method(method);
        self
    }

    pub fn user(&mut self, user: &str) -> &mut Self {
        self.auth_token = Some(format!("Token Dev::User:{}", user));
        self
    }

    pub fn exec(&self) -> apiserver_engine::test_util::Response {
        const ENDPOINT: &str = "http://localhost:1779";
        let url = format!(
            "{}{}",
            ENDPOINT,
            self.builder.action.clone().expect("URL not provided")
        );
        let request = if self.builder.body.is_empty() {
            if self.builder.method == Some(apiserver_engine::test_util::Method::Delete) {
                self.client.delete(&url)
            } else {
                self.client.get(&url)
            }
        } else {
            self.client
                .post(&url)
                .body(serde_json::to_string(&self.builder.body).expect("failed to serialize body"))
        };

        let request = request
            .header(
                "Authorization",
                self.auth_token
                    .clone()
                    .unwrap_or_else(|| "Token Dev::root".to_string()),
            )
            .header("Content-Type", "application/json");

        let response = request.send().unwrap();
        if response
            .headers()
            .get("Content-Type")
            .map(|header_value| header_value.as_bytes())
            != Some(b"application/json")
        {
            eprintln!(
                "Apiserver returned non-json: {:?}",
                response.headers().get("Content-Type")
            );
            eprintln!("Response: {}", response.text().unwrap_or_default());
            panic!()
        }
        let body = response.text().unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        apiserver_engine::test_util::Response(body)
    }
}

impl Default for RequestBuilder {
    fn default() -> RequestBuilder {
        RequestBuilder::new()
    }
}
