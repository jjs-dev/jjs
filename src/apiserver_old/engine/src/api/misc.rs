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

#[get("/system/api-version")]
pub(crate) fn route_get_api_version() -> Json<ApiVersion> {
    Json(ApiVersion { major: 0, minor: 0 })
}

#[get("/system/is-dev")]
pub(crate) fn route_is_dev(ctx: Context) -> Json<bool> {
    Json(matches!(ctx.config().env, crate::config::Env::Dev))
}
