impl super::User {
    pub(crate) fn from_pg_row(row: tokio_postgres::Row) -> super::User {
        Self {
            id: row.get("id"),
            username: row.get("username"),
            groups: row.get("groups"),
            password_hash: row.get("password_hash"),
        }
    }
}
