/// Allows access to public data
use super::{
    context, is_contest_running_at, is_user_contest_sudo, load_participation, resource_ident,
    Action, Operation, Outcome, ResourceKind, Rule, RuleRet,
};
use anyhow::Context as _;
use futures::future::FutureExt as _;
use log::debug;

pub(super) struct ContestViewRule {
    en_cx: context::EntityContext,
}

impl ContestViewRule {
    pub(super) fn new(en_cx: context::EntityContext) -> ContestViewRule {
        ContestViewRule { en_cx }
    }

    async fn do_authorize_operation(self, op: Operation) -> anyhow::Result<Option<Outcome>> {
        if op.action != Action::Get && op.action != Action::List {
            return Ok(None);
        }
        if op.resource_kind == ResourceKind::CONTEST {
            let contest_id = op.conditions.get::<resource_ident::ContestId>().unwrap();
            let contest = self
                .en_cx
                .entities()
                .find::<entity::Contest>(&contest_id.0)
                .unwrap();
            if contest.anon_visible {
                return Ok(Some(Outcome::Allow));
            }
            // TODO: use structured names
            if op.user_info.name == "Global/Guest" {
                return Ok(Some(Outcome::Deny {
                    reason: "contest is hidden from anonymous users".to_string(),
                }));
            }
            if contest.unregistered_visible {
                return Ok(Some(Outcome::Allow));
            }
        }
    }
}
