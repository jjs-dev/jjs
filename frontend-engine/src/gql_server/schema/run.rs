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

    fn last_invoke_dir(&self, ctx: &Context) -> ApiResult<PathBuf> {
        let rejudge_id = self.lookup(ctx)?.rejudge_id;
        let f = format!("i-{}", rejudge_id);
        Ok(self.data_dir(ctx).join(f))
    }

    fn lookup(&self, ctx: &Context) -> ApiResult<db::schema::Run> {
        ctx.db.run_load(self.id).internal(ctx)
    }
}

#[derive(GraphQLInputObject, Copy, Clone)]
pub(crate) struct RunProtocolFilterParams {
    /// If false, compilation logs will be excluded
    compile_log: bool,
    /// If false, test data will be excluded for all tests
    test_data: bool,
    /// If false, solution stdout&stderr will be excluded for all tests
    output: bool,
}

fn filter_protocol(proto: &mut serde_json::Value, filter: RunProtocolFilterParams) {
    let proto = match proto.as_object_mut() {
        Some(p) => p,
        None => return,
    };
    if !filter.compile_log {
        proto.remove("compile_stdout");
        proto.remove("compile_stderr");
    }
    if let Some(tests) = proto.get_mut("tests") {
        if let Some(tests) = tests.as_array_mut() {
            for test in tests {
                if let Some(test) = test.as_object_mut() {
                    if !filter.output {
                        test.remove("test_stdout");
                        test.remove("test_stderr");
                    }
                    if !filter.test_data {
                        test.remove("test_stdin");
                    }
                }
            }
        }
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
    fn source(&self, ctx: &Context) -> ApiResult<Option<String>> {
        let source_path = self.data_dir(ctx).join("source");
        let source = std::fs::read(source_path).ok();
        let source = source.as_ref().map(base64::encode);
        Ok(source)
    }

    /// Returns run build artifact as base64-encoded string
    fn binary(&self, ctx: &Context) -> ApiResult<Option<String>> {
        let binary_path = self.data_dir(ctx).join("build");
        let binary = std::fs::read(binary_path).ok();
        let binary = binary.as_ref().map(base64::encode);
        Ok(binary)
    }

    /// Returns invocation protocol as JSON string
    fn invocation_protocol(
        &self,
        ctx: &Context,
        filter: RunProtocolFilterParams,
    ) -> ApiResult<Option<String>> {
        let path = self.last_invoke_dir(ctx)?.join("log.json");
        let protocol = std::fs::read(path).ok();
        match protocol {
            Some(protocol) => {
                let protocol = String::from_utf8(protocol).internal(ctx)?;
                let mut protocol = serde_json::from_str(&protocol).internal(ctx)?;
                filter_protocol(&mut protocol, filter);
                let protocol = serde_json::to_string(&protocol).internal(ctx)?;
                Ok(Some(protocol))
            }
            None => Ok(None),
        }
    }
}
