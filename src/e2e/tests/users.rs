fn check_login_and_password(login: &str, password: &str) {
    e2e::RequestBuilder::new()
        .action("/users")
        .var("login", login)
        .var("password", password)
        .exec()
        .unwrap_ok();
    e2e::RequestBuilder::new()
        .action("/auth/simple")
        .var("login", login)
        .var("password", password)
        .exec()
        .unwrap_ok();
}

#[test]
fn test_unicode() {
    check_login_and_password("猫鯉", "ありがとうございまので大丈夫");
    check_login_and_password("💻🌐", "🔑");
}
