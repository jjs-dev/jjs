use super::{
    context, is_contest_running_at, is_user_contest_sudo, load_participation, resource_ident,
    Action, Operation, Outcome, ResourceKind, Rule, RuleRet,
};
use anyhow::Context as _;
use futures::future::FutureExt as _;
use log::debug;
use std::rc::Rc;

#[derive(Clone)]
pub(super) struct SubmitRule {
    db_cx: context::DbContext,
    en_cx: context::EntityContext,
}

impl SubmitRule {
    pub(super) fn new(db_cx: context::DbContext, en_cx: context::EntityContext) -> SubmitRule {
        SubmitRule { db_cx, en_cx }
    }
}

impl Rule for SubmitRule {
    fn name(&self) -> &'static str {
        "Submit"
    }

    fn description(&self) -> &'static str {
        "authorizes submitRun operations"
    }

    fn authorize_operation(&self, op: &Rc<Operation>) -> RuleRet {
        let op = op.clone();
        let en_cx = self.en_cx.clone();
        let db_cx = self.db_cx.clone();
        async move {
            if op.resource_kind != ResourceKind::RUN {
                return Ok(None);
            }
            if op.action != Action::Create {
                return Ok(None);
            }
            debug!("SubmitRule: processing {:?}", op);
            if is_user_contest_sudo(&op, &en_cx).await {
                debug!("SubmitRule: allow: user is sudoer on contest");
                return Ok(Some(Outcome::Allow));
            }
    
            let contest_id = op.get_condition::<resource_ident::ContestId>();
    
            let participation = load_participation(&db_cx, &contest_id.0, op.user_info.id)
                .await
                .context("participation lookup failure")?;
    
            let contest = en_cx
                .entities()
                .find::<entity::Contest>(&contest_id.0)
                .ok_or_else(|| anyhow::anyhow!("Unknown contest {}", &contest_id.0))?;
    
            match participation {
                None => Ok(Some(Outcome::Deny {
                    reason: "You are not participating in this contest".to_string(),
                })),
                Some(participation) => match participation.phase() {
                    db::schema::ParticipationPhase::Active => {
                        let is_contest_running =
                            is_contest_running_at(contest, chrono::Utc::now(), &participation);
                        if is_contest_running {
                            debug!("SubmitRule: allow: user is participating");
                            Ok(Some(Outcome::Allow))
                        } else {
                            Ok(Some(Outcome::Deny {
                                reason: "contest is not running".to_string(),
                            }))
                        }
                    }
                    db::schema::ParticipationPhase::__Last => unreachable!(),
                },
            }
        }.boxed_local()
    }
}
