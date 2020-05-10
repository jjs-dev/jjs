use super::{context, resource_ident, Action, Operation, Outcome, ResourceKind, Rule, RuleRet};
use futures::future::FutureExt;
use log::{debug, };
use std::rc::Rc;

pub(super) struct ParticipationRule {
    en_cx: context::EntityContext,
}

impl ParticipationRule {
    pub(super) fn new(en_cx: context::EntityContext) -> ParticipationRule {
        ParticipationRule { en_cx }
    }
}

impl Rule for ParticipationRule {
    fn name(&self) -> &'static str {
        "Participation"
    }

    fn description(&self) -> &'static str {
        "Authorizes requests related to `Participation`s"
    }

    fn authorize_operation(&self, op: &Rc<Operation>) -> RuleRet {
        let op = op.clone();
        let en_cx = self.en_cx.clone();
        async move {
            if op.resource_kind != ResourceKind::CONTEST_PARTICIPATION {
                return Ok(None);
            }
            if op.action != Action::Get && op.action != Action::Patch {
                return Ok(None);
            }

            match op.action {
                Action::Get => {
                    debug!("Allow: get request");
                    Ok(Some(Outcome::Allow))
                }
                Action::Patch => {
                    let contest_id = op.get_condition::<resource_ident::ContestId>();
                    let contest = match en_cx.entities().find::<entity::Contest>(&contest_id.0) {
                        Some(contest) => contest,
                        None => {
                            return Ok(Some(Outcome::Deny {
                                reason: format!("contest {} not found", contest_id.0),
                            }));
                        }
                    };
                    // TODO this must be rewritten when more participation phases are added
                    // probably, we should take the phase as a condition
                    if super::user_matches_group_list(&op.user_info.groups, &contest.participants) {
                        Ok(Some(Outcome::Allow))
                    } else {
                        Ok(Some(Outcome::Deny {
                            reason: "you are not allowed to take part in this contest".to_string(),
                        }))
                    }
                }
                _ => Ok(None),
            }
        }
        .boxed_local()
    }
}
