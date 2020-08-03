pub mod auth_data;
pub use auth_data::AuthData;

pub mod prelude {
    pub use openapi::client::Sendable;
}
use prelude::Sendable as _;

pub mod models {
    pub use openapi::{
        api_version::ApiVersion, live_status::LiveStatus, miscellaneous::Miscellaneous as Misc,
        run::Run, run_patch::RunPatch, run_submit_simple_params::RunSubmitSimpleParams,
        simple_auth_params::SimpleAuthParams, toolchain::Toolchain,
    };
}
use anyhow::Context as _;
use openapi::client::{ApiClient as ApiClientTrait, Client as RawClient};
use std::sync::Arc;
use tracing::instrument;

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

/// auth-data does not necessarily contain token. For example, user can specify
/// login and password instead. This function is responsible for converting
/// any possible AuthKind to token.
async fn obtain_token(ad: AuthData) -> anyhow::Result<String> {
    let mut conf = openapi::client::ClientConfiguration::new();
    conf.set_base_url(&ad.endpoint);
    match ad.auth {
        auth_data::AuthKind::Token(tok) => Ok(tok.token),
        auth_data::AuthKind::LoginAndPassword(lp) => {
            let client = RawClient::new(conf);
            let auth = models::SimpleAuthParams::login()
                .login(lp.login)
                .password(lp.password)
                .send(&client)
                .await?
                .object;
            Ok(auth.token)
        }
    }
}

/// Establishes connection to JJS API using given AuthData
pub async fn from_auth_data(ad: AuthData) -> anyhow::Result<ApiClient> {
    let mut configuration = openapi::client::ClientConfiguration::new();

    configuration.set_base_url(&ad.endpoint);

    let token = obtain_token(ad).await.context("failed to obtain token")?;

    let api_key = openapi::client::ApiKey {
        header_name: "Authorization".to_string(),
        key: format!("Bearer {}", &token),
    };
    configuration.set_api_key(api_key);

    let inner = Arc::new(RawClient::new(configuration));
    Ok(ApiClient { inner })
}

/// Establishes connection to JJS API using environment-dependent methods
#[instrument]
pub async fn infer() -> anyhow::Result<ApiClient> {
    let auth_data = AuthData::infer().await.context("AuthData not found")?;
    from_auth_data(auth_data).await.context("connection error")
}
