mod common;

use common::util;

use serde_json::json;

/// Smoke test
#[test]
fn test_smoke_ops() {
    let env = common::Env::new("SmokeOps");
    let res = env.exec_ok(
        "
query GetApiVersion {
    apiVersion
}
    ",
    );
    assert_eq!(
        res,
        json!({
            "apiVersion": "0.0"
        })
    )
}

/// tests various operations with user
#[test]
fn test_user_ops() {
    let env = common::Env::new("UserOps");
    let res = env.exec_ok(
        r#"
mutation CreateAUser {
    createUser(login: "JonSnow", password: "VerySecretPass", groups: []) {
        login
    }
}
    "#,
    );
    assert_eq!(
        res,
        json!({
            "createUser": {
                "login": "JonSnow"
            }
        })
    );

    let res = env.exec_err(
        r#"
mutation CreateSameUserAgain {
    createUser(login: "JonSnow", password: "VerySecretPass", groups: []) {
        login
    }
}
        "#,
    );
    assert_eq!(res.len(), 1);
    util::check_error(&res[0], "UserAlreadyExists");
}

///  tests operations with run
///  Since it is not end-to-end test, it doesn't check judging
#[test]
fn test_runs_ops() {
    let env = common::EnvBuilder::new()
        .toolchain(cfg::Toolchain {
            title: "C++".to_string(),
            name: "cpp".to_string(),
            filename: "source.cpp".to_string(),
            build_commands: vec![],
            run_command: Default::default(),
            limits: Default::default(),
        })
        .build("runs_ops");

    let res = env.exec_ok(
        r#"
query GetNonExistingRun {
    runs(id: 0) {
        id
    }
}
    "#,
    );
    assert_eq!(
        res,
        json!({
            "runs": []
        })
    );

    static RUN_TEXT: &str = r#"
#include <cstdio>
int main() {
    int a, b;
    scanf(" % d % d", &a, &b);
    printf(" % d", a + b);
}
"#;
    let run_encoded = base64::encode(RUN_TEXT);

    let res = env.exec_ok_with_vars(
        r#"
mutation CreateRun($runCode: String!) {
    submitSimple(toolchain: "cpp", runCode: $runCode, problem: "A", contest: "TODO") {
        id
    }
}
    "#,
        &json!({ "runCode": &run_encoded }),
    );
    assert_eq!(
        res,
        json!({
            "submitSimple": {
                "id": 0
            }
        })
    );

    let res = env.exec_ok(
        r#"
query GetRun {
    runs(id: 0) {
        source
    }
}
    "#,
    );
    let res = res
        .get("runs")
        .unwrap()
        .get(0)
        .unwrap()
        .get("source")
        .unwrap()
        .as_str()
        .unwrap();
    let res = base64::decode(res).unwrap();
    let res = String::from_utf8(res).unwrap();
    assert_eq!(res, RUN_TEXT);
}
