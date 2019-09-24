pub struct RequestBuilder {
    builder: frontend_engine::test_util::RequestBuilder,
    auth_token: Option<String>,
    client: reqwest::Client,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            builder: frontend_engine::test_util::RequestBuilder::new(),
            auth_token: None,
            client: reqwest::Client::new(),
        }
    }

    pub fn vars(&mut self, v: &serde_json::Value) -> &mut Self {
        self.builder.vars(v);
        self
    }

    pub fn operation(&mut self, op: &str) -> &mut Self {
        self.builder.operation(op);
        self
    }

    pub fn exec(&self) -> frontend_engine::test_util::Response {
        let body = self.builder.to_query();
        let request = self
            .client
            .post("http://localhost:1779/graphql")
            .body(body)
            .header(
                "X-Jjs-Auth",
                self.auth_token
                    .clone()
                    .unwrap_or_else(|| "Dev root".to_string())
                    .to_string(),
            )
            .header("Content-Type", "application/json");

        let mut response = request.send().unwrap();
        if response.status() != 200 {
            eprintln!("Frontend returned non-200: {:?}", response.status());
            eprintln!("Response: {}", response.text().unwrap_or_default());
            panic!()
        }
        let body = response.text().unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        frontend_engine::test_util::Response(body)
    }
}
