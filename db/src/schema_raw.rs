table! {
    use super::*;

    invocation_requests (id) {
        id -> Int4,
        run_id -> Int4,
        invoke_revision -> Int4,
    }
}

table! {
    use super::*;

    runs (id) {
        id -> Int4,
        toolchain_id -> Varchar,
        status_code -> Varchar,
        status_kind -> Varchar,
        problem_id -> Varchar,
        score -> Int4,
        rejudge_id -> Int4,
    }
}

table! {
    use super::*;

    users (id) {
        id -> Uuid,
        username -> Varchar,
        password_hash -> Bpchar,
        groups -> Array<Text>,
    }
}

joinable!(invocation_requests -> runs (run_id));

allow_tables_to_appear_in_same_query!(
    invocation_requests,
    runs,
    users,
);
