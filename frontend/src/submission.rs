use crate::frontend_api::submission::*;
use db::submission::Submissions;
fn api_fun_s8n_declare<'a>(q: &'a frontend_api::submission::DeclareRequest, ctx: crate::ApiFunContext<'a>) {
    ctx.db.submissions.create_submission(&q.toolchain, &q.check_sum.digest)
}