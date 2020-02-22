fn check_login_and_password(login: &str, password: &str) {
    e2e::RequestBuilder::new()
        .operation(
            r#"
mutation CreateUser($login: String!, $password: String!) {
    createUser(login: $login, password: $password, groups: []) {
        id
    }
}
"#,
        )
        .var("login", &login.into())
        .var("password", &password.into())
        .exec()
        .unwrap_ok();
    e2e::RequestBuilder::new()
        .operation(
            r#"
mutation LogIn($login: String!, $password: String!) {
    authSimple(login: $login, password: $password) {
        data
    }
}
"#,
        )
        .var("login", &login.into())
        .var("password", &password.into())
        .exec()
        .unwrap_ok();
}

#[test]
fn test_unicode() {
    check_login_and_password("猫鯉", "ありがとうございまので大丈夫");
    check_login_and_password("💻🌐", "🔑");
    check_login_and_password("死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死", "死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死死");
    check_login_and_password("𘕣𗣳\"𘕪𘛏𗄐𗷔 𗙫𗟩𗓮𘔔\"", "𗫮");
}
