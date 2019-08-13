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

pub(super) fn get_contests(ctx: &Context) -> ApiResult<Vec<schema::Contest>> {
    let contest_cfg = &ctx.cfg.contests[0];
    Ok(vec![schema::Contest {
        title: contest_cfg.title.clone(),
        id: "TODO".to_string(),
    }])
}
