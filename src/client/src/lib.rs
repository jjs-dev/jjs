pub use openapi::models;
pub type ApiClient = openapi::apis::DefaultApiClient<hyper::client::HttpConnector>;
pub use openapi::apis::DefaultApi as Api;
pub type Error = openapi::apis::Error<serde_json::Value>;
pub fn connect() -> ApiClient {
    let hyper_client = hyper::client::Client::new();
    let mut configuration = openapi::apis::configuration::Configuration::new(hyper_client);
    configuration.base_path =
        std::env::var("JJS_API").unwrap_or_else(|_| "http://localhost:1779".to_string());
    configuration.api_key = Some(openapi::apis::configuration::ApiKey {
        key: std::env::var("JJS_TOKEN").unwrap_or_else(|_| "Dev::root".to_string()),
        prefix: Some("Token".to_string()),
    });
    openapi::apis::DefaultApiClient::new(std::rc::Rc::new(configuration))
}
