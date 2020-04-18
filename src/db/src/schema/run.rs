impl super::Run {
    pub(crate) fn from_pg_row(row: tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            toolchain_id: row.get("toolchain_id"),
            problem_id: row.get("problem_id"),
            contest_id: row.get("contest_id"),
            user_id: row.get("user_id"),
            rejudge_id: row.get("rejudge_id"),
        }
    }
}
