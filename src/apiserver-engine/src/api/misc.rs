use super::prelude::*;

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct ApiVersion {
    /// MAJOR component
    major: u16,
    /// MINOR component
    minor: u16,
}

impl ApiObject for ApiVersion {
    fn name() -> &'static str {
        "ApiVersion"
    }
}

async fn route_get_api_version() -> Json<ApiVersion> {
    Json(ApiVersion { major: 0, minor: 0 })
}

async fn route_is_dev(cx: ConfigContext) -> Json<bool> {
    Json(matches!(cx.config().env, crate::config::Env::Dev))
}

pub(crate) fn register_routes(c: &mut web::ServiceConfig) {
    c.route("/system/api-version", web::get().to(route_get_api_version))
        .route("/system/is-dev", web::get().to(route_is_dev));
}
