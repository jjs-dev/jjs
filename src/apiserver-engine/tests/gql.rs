mod common;

use serde_json::json;

/// Smoke test
#[tokio::test]
async fn test_smoke_ops() {
    let env = common::Env::new().await;
    let res = env
        .req()
        .action("/system/api-version")
        .exec()
        .await
        .unwrap_ok();
    assert_eq!(
        res,
        json!({
            "major": 0,
            "minor": 0
        })
    )
}

///  tests operations with run
///  Since it is not end-to-end test, it doesn't check judging
#[tokio::test]
async fn test_runs_ops() {
    let env = common::EnvBuilder::new()
        .toolchain(entity::Toolchain {
            title: "C++".to_string(),
            name: "cpp".to_string(),
            filename: "source.cpp".to_string(),
            build_commands: vec![],
            run_command: Default::default(),
            limits: Default::default(),
            env: std::collections::HashMap::new(),
            env_blacklist: vec![],
            env_passing: true,
        })
        .contest(entity::Contest {
            anon_visible: true,
            title: "test contest".to_string(),
            id: "main".to_string(),
            problems: vec![entity::entities::contest::ProblemBinding {
                name: "a-plus-b".to_string(),
                code: "A".to_string(),
            }],
            group: vec!["Participants".to_string()],
            judges: vec!["Judges".to_string()],
            unregistered_visible: true,
            duration: None,
            start_time: None,
            end_time: None,
            is_virtual: false,
        })
        .build()
        .await;

    let res = env.req().action("/runs/0").exec().await.unwrap_err();
    assert_eq!(res["detail"]["errorCode"], "NotFound");

    static RUN_TEXT: &str = r#"
#include <cstdio>
int main() {
    int a, b;
    scanf("%d %d", &a, &b);
    printf("%d", a + b);
}
"#;
    env.req()
        .action("/contests/main/participation")
        .method(apiserver_engine::test_util::Method::Patch)
        .var("phase", "ACTIVE")
        .exec()
        .await
        .unwrap_ok();
    let run_encoded = base64::encode(RUN_TEXT);

    let res = env
        .req()
        .action("/runs")
        .var("code", json!(run_encoded))
        .var("toolchain", "cpp")
        .var("problem", "A")
        .var("contest", "main")
        .exec()
        .await
        .unwrap_ok();
    assert_eq!(
        res,
        json!({
            "contest_id": "main",
            "id": 0,
            "problem_name": "dev-problem",
            "score": null,
            "status": null,
            "toolchain_name": "cpp"
        })
    );

    let res = env.req().action("/runs/0/source").exec().await.unwrap_ok();
    let res = res.as_str().unwrap();
    let res = base64::decode(res).unwrap();
    let res = String::from_utf8(res).unwrap();
    assert_eq!(res, RUN_TEXT);
}
