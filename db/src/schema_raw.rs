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
