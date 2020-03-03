use super::prelude::*;

pub(super) fn toolchains_list(ctx: &Context) -> ApiResult<Vec<schema::Toolchain>> {
    let res = ctx
        .cfg
        .list::<entity::Toolchain>()
        .map(|tc| schema::Toolchain {
            name: tc.title.clone(),
            id: tc.name.clone(),
        })
        .collect();
    Ok(res)
}

fn describe_contest(c: &entity::Contest) -> schema::Contest {
    schema::Contest {
        title: c.title.clone(),
        name: c.name.clone(),
    }
}

pub(super) fn get_contests(ctx: &Context) -> ApiResult<Vec<schema::Contest>> {
    let res = ctx
        .cfg
        .list::<entity::Contest>()
        .map(describe_contest)
        .collect();
    Ok(res)
}

pub(super) fn get_contest(ctx: &Context, name: &str) -> ApiResult<Option<schema::Contest>> {
    let res = ctx.cfg.find(name).map(describe_contest);
    Ok(res)
}
