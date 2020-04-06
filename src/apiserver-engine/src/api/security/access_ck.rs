use super::Token;

#[derive(Clone, Debug)]
pub(crate) struct Subjects {
    pub(crate) contest: Option<entity::Contest>,
    pub(crate) participation: Option<db::schema::Participation>,
    pub(crate) run: Option<db::schema::Run>,
}

/// Access check service
#[derive(Clone, Debug)]
pub(crate) struct AccessChecker<'a> {
    pub(crate) token: &'a Token,
    pub(crate) cfg: &'a entity::Loader,
    pub(crate) subjects: Subjects,
}

impl AccessChecker<'_> {
    pub(crate) fn is_sudo(&self) -> bool {
        self.token.user_info.name == "Global/Root"
    }
}

impl AccessChecker<'_> {
    pub(crate) fn can_modify_run(&self) -> bool {
        if self.is_contest_sudo() {
            return true;
        }

        self.subjects.run.as_ref().unwrap().user_id == self.token.user_id()
    }

    pub(crate) fn can_view_run(&self) -> bool {
        if self.is_contest_sudo() {
            return true;
        }
        self.subjects.run.as_ref().unwrap().user_id == self.token.user_id()
    }
}

impl AccessChecker<'_> {
    pub(crate) fn can_participate(&self) -> bool {
        if self.is_contest_sudo() {
            return true;
        }
        let mut is_registered = false;
        for registered_group in &self.subjects.contest.as_ref().unwrap().group {
            if self.token.user_info.groups.contains(registered_group) {
                is_registered = true;
                break;
            }
        }
        is_registered
    }

    pub(crate) fn can_submit(&self) -> bool {
        if self.is_contest_sudo() {
            return true;
        }
        match &self.subjects.participation {
            None => false,
            Some(p) => match p.phase() {
                db::schema::ParticipationPhase::Active => is_contest_running_at(
                    self.subjects.contest.as_ref().unwrap(),
                    chrono::Utc::now(),
                    self.subjects.participation.as_ref().unwrap(),
                ),
                db::schema::ParticipationPhase::__Last => unreachable!(),
            },
        }
    }

    fn is_contest_sudo(&self) -> bool {
        if self.is_sudo() {
            return true;
        }
        for judges_group in &self.subjects.contest.as_ref().unwrap().judges {
            if self.token.user_info.groups.contains(judges_group) {
                return true;
            }
        }
        false
    }

    pub(crate) fn select_judge_log_kind(&self) -> invoker_api::valuer_proto::JudgeLogKind {
        use invoker_api::valuer_proto::JudgeLogKind;
        if self.is_contest_sudo() {
            return JudgeLogKind::Full;
        }
        JudgeLogKind::Contestant
    }
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
