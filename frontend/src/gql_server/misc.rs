use super::prelude::*;

pub(super) fn toolchains_list(ctx: &Context) -> ApiResult<Vec<schema::Toolchain>> {
    let res = ctx
        .cfg
        .toolchains
        .iter()
        .enumerate()
        .map(|(i, tc)| schema::Toolchain {
            name: tc.name.clone(),
            id: i as schema::ToolchainId,
        })
        .collect();
    Ok(res)
}
