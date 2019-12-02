table! {
    use super::*;

    invocations (id) {
        id -> Int4,
        invoke_task -> Bytea,
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
        user_id -> Uuid,
    }
}

table! {
    use super::*;

    users (id) {
        id -> Uuid,
        username -> Varchar,
        password_hash -> Nullable<Bpchar>,
        groups -> Array<Text>,
    }
}

joinable!(runs -> users (user_id));

allow_tables_to_appear_in_same_query!(
    invocations,
    runs,
    users,
);
