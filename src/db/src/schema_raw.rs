table! {
    use super::*;

    invocations (id) {
        id -> Int4,
        run_id -> Int4,
        invoke_task -> Bytea,
        state -> Int2,
        outcome -> Jsonb,
    }
}

table! {
    use super::*;

    runs (id) {
        id -> Int4,
        toolchain_id -> Varchar,
        problem_id -> Varchar,
        rejudge_id -> Int4,
        user_id -> Uuid,
        contest_name -> Varchar,
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

joinable!(invocations -> runs (run_id));
joinable!(runs -> users (user_id));

allow_tables_to_appear_in_same_query!(
    invocations,
    runs,
    users,
);
