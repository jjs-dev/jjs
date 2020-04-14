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

async fn route_list(ecx: EntityContext) -> ApiResult<Json<Vec<Toolchain>>> {
    let res = ecx
        .entities()
        .list::<entity::Toolchain>()
        .map(|tc| Toolchain {
            name: tc.title.clone(),
            id: tc.name.clone(),
        })
        .collect();
    Ok(Json(res))
}

pub(crate) fn register_routes(c: &mut web::ServiceConfig) {
    c.route("/toolchains", web::get().to(route_list));
}
