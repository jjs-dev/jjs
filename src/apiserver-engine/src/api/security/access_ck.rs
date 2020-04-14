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

    pub(crate) fn select_judge_log_kind(
        &self,
    ) -> invoker_api::valuer_proto::JudgeLogKind {
        use invoker_api::valuer_proto::JudgeLogKind;
        if self.is_contest_sudo() {
            return JudgeLogKind::Full;
        }
        JudgeLogKind::Contestant
    }
}
