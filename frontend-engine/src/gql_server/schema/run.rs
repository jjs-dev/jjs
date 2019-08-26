use crate::gql_server::{
    prelude::*,
    schema::{InvokeStatusOut, Problem, RunId, Toolchain},
};
use std::path::PathBuf;

pub(crate) struct Run {
    pub id: RunId,
    pub toolchain_name: String,
    pub status: InvokeStatusOut,
    pub score: Option<i32>,
    pub problem_name: String,
}

impl Run {
    fn data_dir(&self, ctx: &Context) -> PathBuf {
        ctx.cfg
            .sysroot
            .join("var/submissions")
            .join(format!("s-{}", self.id))
    }

    fn lookup(&self, ctx: &Context) -> Result<db::schema::Run, db::Error> {
        ctx.db.run_load(self.id)
    }
}

#[juniper::object(Context = Context)]
impl Run {
    fn id(&self) -> RunId {
        self.id
    }

    fn toolchain(&self, ctx: &Context) -> Toolchain {
        ctx.cfg
            .find_toolchain(&self.toolchain_name)
            .expect("toolchain not found")
            .into()
    }

    fn status(&self) -> &InvokeStatusOut {
        &self.status
    }

    fn score(&self) -> Option<i32> {
        self.score
    }

    fn problem(&self, ctx: &Context) -> Problem {
        ctx.cfg
            .find_problem(&self.problem_name)
            .expect("problem not found")
            .into()
    }

    /// Returns run source as base64-encoded string
    fn source(&self, ctx: &Context) -> ApiResult<String> {
        let source_path = self.data_dir(ctx).join("source");
        let source = std::fs::read(source_path).internal(ctx)?;
        let source = base64::encode(&source);
        Ok(source)
    }

    /// Returns run build artifact as base64-encoded string
    fn binary(&self, ctx: &Context) -> ApiResult<String> {
        let binary_path = self
            .data_dir(ctx)
            .join("build");
        let binary = std::fs::read(binary_path).internal(ctx)?;
        let binary = base64::encode(&binary);
        Ok(binary)
    }
}
