mod common;

use common::{Env, RequestBuilder};
use serde_json::json;

const EMPTY_GROUPS: &[String; 0] = &[];

impl RequestBuilder {
    fn create_user(
        &mut self,
        name: &str,
        password: &str,
        groups: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut Self {
        self.action("/users")
            .var("login", name)
            .var("password", password)
            .var(
                "groups",
                groups
                    .into_iter()
                    .map(|x| serde_json::Value::String(x.as_ref().to_string()))
                    .collect::<Vec<_>>(),
            )
    }

    fn submit_staff(&mut self) -> &mut Self {
        self.action("/runs")
            .var("toolchain", "cpp")
            .var("code", "")
            .var("problem", "A")
            .var("contest", "main")
    }
}

impl Env {
    async fn login(&self, login: &str, password: &str) -> String {
        let res = self
            .req()
            .action("/auth/simple")
            .var("login", login)
            .var("password", password)
            .exec()
            .await
            .unwrap_ok();

        res["data"].as_str().unwrap().to_string()
    }
}

/// tests that multiple users with same login are not allowed
#[tokio::test]
async fn test_user_already_exists() {
    let env = Env::new().await;
    let res = env
        .req()
        .create_user("JonSnow", "VerySecretPass", EMPTY_GROUPS)
        .exec()
        .await
        .unwrap_ok();
    assert_eq!(res["login"], json!("JonSnow"));

    let res = env
        .req()
        .create_user("JonSnow", "VerySecretPass", EMPTY_GROUPS)
        .exec()
        .await
        .unwrap_err();
    common::check_error(&res, "UserAlreadyExists");
}

#[tokio::test]
async fn test_groups() {
    let env = Env::new().await;
    let res = env
        .req()
        .create_user("Alice", "pswrd", &["bar"])
        .exec()
        .await
        .unwrap_ok();
    assert_eq!(res["login"], "Alice");
    let token = env.login("Alice", "pswrd").await;
    let err = env
        .req()
        .auth(token)
        .create_user("Bob", "pswrd", &["bar"])
        .exec()
        .await
        .unwrap_err();
    common::check_error(&err, "AccessDenied");
}

#[tokio::test]
async fn test_contests_participations_restrictions() {
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
        .build()
        .await;
    env.req()
        .create_user("Alice", "", &["Participants"])
        .exec()
        .await
        .unwrap_ok();
    env.req()
        .create_user("Bob", "", &["Participants"])
        .exec()
        .await
        .unwrap_ok();
    let alice_token = env.login("Alice", "").await;
    let bob_token = env.login("Bob", "").await;
    let err = env
        .req()
        .submit_staff()
        .auth(&alice_token)
        .exec()
        .await
        .unwrap_err();
    common::check_error(&err, "AccessDenied");
    env.req()
        .auth(&bob_token)
        .action("/contests/main/participation")
        .method(apiserver_engine::test_util::Method::Patch)
        .var("phase", "ACTIVE")
        .exec()
        .await
        .unwrap_ok();
    let err = env
        .req()
        .submit_staff()
        .auth(&alice_token)
        .exec()
        .await
        .unwrap_err();
    common::check_error(&err, "AccessDenied");
    env.req()
        .auth(&bob_token)
        .submit_staff()
        .exec()
        .await
        .unwrap_ok();
}
