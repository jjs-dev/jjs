use std::convert::TryInto;

#[repr(i16)]
#[derive(Debug, postgres_types::FromSql, postgres_types::ToSql)]
pub enum ParticipationPhase {
    Active,
    // in future: Requested, Rejected
    // in future: Disqualified,
    __Last,
}

impl From<ParticipationPhase> for i16 {
    fn from(val: ParticipationPhase) -> Self {
        val as i16
    }
}

impl std::convert::TryFrom<i16> for ParticipationPhase {
    type Error = ();

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        if value < 0 || value >= (ParticipationPhase::__Last as i16) {
            return Err(());
        }
        Ok(unsafe { std::mem::transmute(value) })
    }
}

impl crate::schema::Participation {
    pub(crate) fn from_pg_row(row: tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            user_id: row.get("user_id"),
            contest_id: row.get("contest_id"),
            phase: row.get("phase"),
            virtual_contest_start_time: row.get("virtual_contest_start_time"),
        }
    }
}

impl crate::schema::Participation {
    pub fn phase(&self) -> ParticipationPhase {
        self.phase.try_into().expect("invalid phase")
    }

    pub fn virtual_contest_start_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.virtual_contest_start_time
            .map(|t| chrono::DateTime::from_utc(t, chrono::Utc))
    }

    pub fn mock_new() -> Self {
        Self {
            phase: 0,
            id: 0,
            contest_id: "".to_string(),
            user_id: uuid::Uuid::nil(),
            virtual_contest_start_time: None,
        }
    }
}

impl crate::schema::NewParticipation {
    pub fn set_phase(&mut self, phase: ParticipationPhase) -> &mut Self {
        self.phase = phase.into();
        self
    }

    pub fn set_virtual_contest_start_time(
        &mut self,
        time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> &mut Self {
        self.virtual_contest_start_time = time.map(|t| t.naive_utc());
        self
    }
}
