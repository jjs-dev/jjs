table! {
    use super::*;

    submissions (id) {
        id -> Int4,
        toolchain_id -> Varchar,
        state -> Submission_state,
        status_code -> Varchar,
        status_kind -> Varchar,
        problem_name -> Varchar,
        judge_revision -> Int4,
    }
}

table! {
    use super::*;

    users (id) {
        id -> Int4,
        username -> Varchar,
        password_hash -> Bpchar,
        groups -> Array<Text>,
    }
}

allow_tables_to_appear_in_same_query!(
    submissions,
    users,
);
