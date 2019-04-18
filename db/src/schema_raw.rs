table! {
    use super::*;

    submissions (id) {
        id -> Int4,
        toolchain_id -> Varchar,
        state -> Submission_state,
        status -> Varchar,
        status_kind -> Varchar,
    }
}

table! {
    use super::*;

    users (id) {
        id -> Int4,
        username -> Varchar,
        password_hash -> Varchar,
    }
}

allow_tables_to_appear_in_same_query!(
    submissions,
    users,
);
