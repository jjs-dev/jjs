table! {
    use super::*;

    invokation_requests (id) {
        id -> Int4,
        submission_id -> Int4,
        invoke_revision -> Int4,
    }
}

table! {
    use super::*;

    submissions (id) {
        id -> Int4,
        toolchain_id -> Varchar,
        state -> Submission_state,
        status_code -> Varchar,
        status_kind -> Varchar,
        problem_name -> Varchar,
        score -> Int4,
        rejudge_id -> Int4,
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

joinable!(invokation_requests -> submissions (submission_id));

allow_tables_to_appear_in_same_query!(
    invokation_requests,
    submissions,
    users,
);
