use super::{context, resource_ident, Operation, Outcome, ResourceKind, Rule, RuleRet};
use anyhow::Context as _;
use futures::future::FutureExt;
use log::debug;
use std::rc::Rc;

pub(super) struct RunRule {
    db_cx: context::DbContext,
    en_cx: context::EntityContext,
}

impl RunRule {
    pub(super) fn new(db_cx: context::DbContext, en_cx: context::EntityContext) -> RunRule {
        RunRule { db_cx, en_cx }
    }
}

impl Rule for RunRule {
    fn name(&self) -> &'static str {
        "RunRule"
    }

    fn description(&self) -> &'static str {
        "authorizes read requests on /runs"
    }

    fn authorize_operation(&self, op: &Rc<Operation>) -> RuleRet {
        let op = op.clone();
        let db_cx = self.db_cx.clone();
        let en_cx = self.en_cx.clone();
        async move {
            if op.resource_kind != ResourceKind::RUN
                && op.resource_kind != ResourceKind::RUN_PROTOCOL
            {
                return Ok(None);
            }
            if super::is_user_contest_sudo(&op, &en_cx).await {
                debug!("allow: user has full rights on this contest");
                return Ok(Some(Outcome::Allow));
            }
            let run_id = op.get_condition::<resource_ident::RunId>();

            let run_data = db_cx
                .db()
                .run_load(run_id.0)
                .await
                .context("can not fetch run from db")?;

            if run_data.user_id == op.user_info.id {
                debug!("allow: user is run author");
                Ok(Some(Outcome::Allow))
            } else {
                Ok(Some(Outcome::Deny {
                    reason: "this is not your run".to_string()
                }))
            }
        }
        .boxed_local()
    }
}
