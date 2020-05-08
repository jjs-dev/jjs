/// Allows access to public data
use super::{context, resource_ident, Action, Operation, Outcome, ResourceKind, Rule, RuleRet};
use futures::future::FutureExt as _;
use log::debug;

#[derive(Clone)]
pub(super) struct ContestViewRule {
    en_cx: context::EntityContext,
}

impl ContestViewRule {
    pub(super) fn new(en_cx: context::EntityContext) -> ContestViewRule {
        ContestViewRule { en_cx }
    }

    fn do_authorize_operation(&self, op: &Operation) -> anyhow::Result<Option<Outcome>> {
        if op.action != Action::Get && op.action != Action::List {
            return Ok(None);
        }
        if op.resource_kind == ResourceKind::CONTEST {
            debug!("processing {:?}", op);
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

            if !super::user_matches_group_list(&op.user_info.groups, &contest.participants) {
                return Ok(Some(Outcome::Deny {
                    reason: "you can not view this contest".to_string(),
                }));
            }

            Ok(Some(Outcome::Allow))
        } else {
            Ok(None)
        }
    }
}

impl Rule for ContestViewRule {
    fn name(&self) -> &'static str {
        "ViewContest"
    }

    fn description(&self) -> &'static str {
        "authorizes read-only contest data requests"
    }

    fn authorize_operation(&self, op: &Operation) -> RuleRet {
        futures::future::ready(self.do_authorize_operation(op)).boxed_local()
    }
}
