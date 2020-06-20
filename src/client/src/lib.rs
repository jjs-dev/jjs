use openapi::client::{ApiClient as ApiClientTrait, Client as RawClient};
use std::sync::Arc;

pub mod prelude {
    pub use openapi::client::Sendable;
}

pub mod models {
    pub use openapi::{api_version, miscellaneous, run, run_patch, run_submit_simple_params};
}
#[derive(Clone)]
pub struct ApiClient {
    inner: Arc<RawClient>,
}

#[async_trait::async_trait]
impl ApiClientTrait for ApiClient {
    type Request = <RawClient as ApiClientTrait>::Request;
    type Response = <RawClient as ApiClientTrait>::Response;

    fn request_builder(&self, method: http::Method, rel_path: &str) -> Self::Request {
        self.inner.request_builder(method, rel_path)
    }

    async fn make_request(
        &self,
        req: Self::Request,
    ) -> Result<Self::Response, openapi::client::ApiError<Self::Response>> {
        self.inner.make_request(req).await
    }
}

pub type Error = openapi::client::ApiError<serde_json::Value>;

/// Establishes connection to JJS API using environment-dependent methods
pub async fn connect() -> anyhow::Result<ApiClient> {
    let mut configuration = openapi::client::ClientConfiguration::new();

    let base_path =
        std::env::var("JJS_API").unwrap_or_else(|_| "http://localhost:1779".to_string());
    configuration.set_base_url(base_path);

    let api_key = std::env::var("JJS_TOKEN").unwrap_or_else(|_| "Dev::root".to_string());
    let api_key = openapi::client::ApiKey {
        header_name: "Authorization".to_string(),
        key: format!("Token {}", &api_key),
    };
    configuration.set_api_key(api_key);

    let inner = Arc::new(RawClient::new(configuration));
    Ok(ApiClient { inner })
}
