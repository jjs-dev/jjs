pub fn get_password_hash(password: &str) -> String {
    use digest::Digest;
    let digest = sha3::Sha3_512::digest(password.as_bytes());
    let digest = format!("{:x}", digest);
    digest
}

pub fn check_password_hash(password: &str, expected_hash: &str) -> bool {
    let actual_hash = get_password_hash(password);
    constant_time_eq::constant_time_eq(expected_hash.as_bytes(), actual_hash.as_bytes())
}