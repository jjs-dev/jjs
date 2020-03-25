mod common;

use common::{Env, RequestBuilder};
use serde_json::json;

const EMPTY_GROUPS: &[String; 0] = &[];

impl<'a> RequestBuilder<'a> {
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
    let env = Env::new("UserOps").await;
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
    let env = Env::new("Groups").await;
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
