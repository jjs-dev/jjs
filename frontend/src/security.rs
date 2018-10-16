use std::time::{SystemTime, UNIX_EPOCH, Duration};

fn conv_to_ms(d: Duration) -> u64 {
    d.as_secs() * 1000 + (d.subsec_nanos() as u64) / 1_000_000
}

pub struct Token {
    pub key: u64,
    pub expires: u64,
    pub user_id: String,
}

impl Token {
    pub fn create_for_user(user_id: String, timeout: Duration) -> Token {
        let cur_time = conv_to_ms(SystemTime::now().duration_since(UNIX_EPOCH).unwrap());
        let expires = cur_time + conv_to_ms(timeout);
        let key = rand::random();
        //let key = uuid::Uuid::new_v4();
        Token {
            key,
            expires,
            user_id,
        }
    }
}