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
        self.operation(
            r#"
mutation CreateUser($login: String!, $password: String!, $groups: [String!]!) {
    createUser(login: $login, password: $password, groups: $groups) {
        login
    }
}
"#,
        )
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
    fn login(&self, login: &str, password: &str) -> String {
        let res = self
            .req()
            .operation(
                "
mutation LogIn($login: String!, $password: String!) {
    authSimple(login:$login, password: $password) {
        data
    }
}        
        ",
            )
            .var("login", login)
            .var("password", password)
            .exec()
            .unwrap_ok();

        res.pointer("/authSimple/data")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }
}

/// tests that multiple users with same login are not allowed
#[test]
fn test_user_already_exists() {
    let env = Env::new("UserOps");
    let res = env
        .req()
        .create_user("JonSnow", "VerySecretPass", EMPTY_GROUPS)
        .exec()
        .unwrap_ok();
    assert_eq!(
        res,
        json!({
            "createUser": {
                "login": "JonSnow"
            }
        })
    );

    let res = env
        .req()
        .create_user("JonSnow", "VerySecretPass", EMPTY_GROUPS)
        .exec()
        .unwrap_errs();
    assert_eq!(res.len(), 1);
    common::check_error(&res[0], "UserAlreadyExists");
}

#[test]
fn test_groups() {
    let env = Env::new("Groups");
    let res = env
        .req()
        .create_user("Alice", "pswrd", &["bar"])
        .exec()
        .unwrap_ok();
    assert_eq!(
        res,
        json!({
            "createUser": {
                "login": "Alice"
            }
        })
    );
    let token = env.login("Alice", "pswrd");
    let errs = env
        .req()
        .auth(token)
        .create_user("Bob", "pswrd", &["bar"])
        .exec()
        .unwrap_errs();
    assert_eq!(errs.len(), 1);
    common::check_error(&errs[0], "AccessDenied");
}
