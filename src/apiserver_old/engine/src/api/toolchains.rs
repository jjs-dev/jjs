use super::prelude::*;

#[derive(Serialize, Deserialize, JsonSchema)]
pub(crate) struct Toolchain {
    /// Human readable name, e.g. "GCC C++ v9.1 with sanitizers enables"
    pub name: String,
    /// Internal name, e.g. "cpp.san.9.1"
    pub id: schema::ToolchainId,
}

impl ApiObject for Toolchain {
    fn name() -> &'static str {
        "Toolchain"
    }
}

impl<'a> From<&'a entity::entities::toolchain::Toolchain> for Toolchain {
    fn from(tc: &'a entity::entities::toolchain::Toolchain) -> Self {
        Self {
            name: tc.title.clone(),
            id: tc.name.clone(),
        }
    }
}

#[get("/toolchains")]
pub(crate) fn route_list(ctx: Context) -> ApiResult<Json<Vec<Toolchain>>> {
    let res = ctx
        .cfg
        .list::<entity::Toolchain>()
        .map(|tc| Toolchain {
            name: tc.title.clone(),
            id: tc.name.clone(),
        })
        .collect();
    Ok(Json(res))
}
