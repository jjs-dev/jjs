use super::{resource_ident, Action, Operation, Outcome, ResourceKind, Rule, RuleRet};
use crate::api::context;
mod submit;
mod contest_view;

pub(crate) fn install(
    builder: &mut super::AuthorizerBuilder,
    db_cx: context::DbContext,
    en_cx: context::EntityContext,
) {
    let submit_rule = submit::SubmitRule::new(db_cx, en_cx);
    builder.add_rule(Box::new(submit_rule));
}

async fn load_participation(
    dcx: &context::DbContext,
    contest_id: &str,
    user_id: uuid::Uuid,
) -> anyhow::Result<Option<db::schema::Participation>> {
    dcx.db().part_lookup(user_id, &contest_id).await
}

fn is_user_sudo(op: &Operation) -> bool {
    op.user_info.name == "Global/Root"
}

async fn is_user_contest_sudo(op: &Operation, ecx: &context::EntityContext) -> bool {
    if is_user_sudo(op) {
        return true;
    }
    let contest_id = op.conditions.get::<resource_ident::ContestId>();
    let contest_id = contest_id.unwrap();
    let contest: &entity::Contest = ecx.entities().find(&contest_id.0).unwrap();
    for judges_group in &contest.judges {
        if op.user_info.groups.contains(judges_group) {
            return true;
        }
    }
    false
}

fn is_contest_running_at(
    contest: &entity::Contest,
    moment: chrono::DateTime<chrono::Utc>,
    participation: &db::schema::Participation,
) -> bool {
    if contest.is_virtual {
        let time_since_beginning = moment - participation.virtual_contest_start_time().unwrap();
        match contest.duration {
            Some(contest_duration) => {
                time_since_beginning <= chrono::Duration::from_std(contest_duration).unwrap()
                    && time_since_beginning.num_milliseconds() >= 0
            }
            None => true,
        }
    } else {
        match contest.end_time {
            Some(end_time) => moment < end_time,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::schema::Participation;
    use entity::Contest;

    fn mktime(h: u32, m: u32, s: u32) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_utc(
            chrono::NaiveDate::from_ymd(2001, 1, 1).and_hms(h, m, s),
            chrono::Utc,
        )
    }

    fn mkpart_nonvirt() -> Participation {
        Participation::mock_new()
    }

    #[test]
    fn test_is_contest_running_at() {
        assert!(is_contest_running_at(
            &Contest {
                is_virtual: false,
                start_time: Some(mktime(10, 0, 0)),
                end_time: Some(mktime(14, 0, 0)),
                ..Default::default()
            },
            mktime(12, 0, 0),
            &mkpart_nonvirt()
        ));

        assert!(!is_contest_running_at(
            &Contest {
                is_virtual: false,
                start_time: Some(mktime(10, 0, 0)),
                end_time: Some(mktime(14, 0, 0)),
                ..Default::default()
            },
            mktime(16, 0, 0),
            &mkpart_nonvirt()
        ));
    }
}
