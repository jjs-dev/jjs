pub mod models {
    pub use openapi::{api_version, run, run_submit_simple_params};
}
pub type ApiClient = openapi::client::Client;
pub type Error = openapi::client::ApiError<serde_json::Value>;
pub fn connect() -> ApiClient {
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

    ApiClient::new(configuration)
}
