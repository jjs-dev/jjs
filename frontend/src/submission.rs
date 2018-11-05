use crate::frontend_api::submission::*;
use db::submission::Submissions;
pub fn api_fun_s8n_declare<'a>(q: &'a frontend_api::submission::DeclareRequest,
                               ctx: crate::ApiFunContext<'a>)
    -> crate::ApiResult<frontend_api::submission::DeclareResult> {
    ctx.db.submissions.create_submission(&q.toolchain, &q.check_sum.digest);
    Ok(Ok(frontend_api::submission::DeclareSuccess{
        upload_token: 0,
    }))
}