mod common;

use serde_json::json;

/// Smoke test
#[test]
fn test_smoke_ops() {
    let env = common::Env::new("SmokeOps");
    let res = env
        .req()
        .operation(
            "
query GetApiVersion {
    apiVersion
}
    ",
        )
        .exec()
        .unwrap_ok();
    assert_eq!(
        res,
        json!({
            "apiVersion": "0.0"
        })
    )
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

    let res = env
        .req()
        .operation(
            r#"
query GetNonExistingRun {
    runs(id: 0) {
        id
    }
}
    "#,
        )
        .exec()
        .unwrap_ok();
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

    let res = env
        .req()
        .operation(
            r#"
mutation CreateRun($runCode: String!) {
    submitSimple(toolchain: "cpp", runCode: $runCode, problem: "A", contest: "TODO") {
        id
    }
}
    "#,
        )
        .var("runCode", json!(run_encoded))
        .exec()
        .unwrap_ok();
    assert_eq!(
        res,
        json!({
            "submitSimple": {
                "id": 0
            }
        })
    );

    let res = env
        .req()
        .operation(
            r#"
query GetRun {
    runs(id: 0) {
        source
    }
}
    "#,
        )
        .exec()
        .unwrap_ok();
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
