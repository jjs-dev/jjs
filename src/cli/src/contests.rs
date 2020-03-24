use client::Api as _;
use serde_json::Value;

pub async fn exec(common: &super::CommonParams) -> Value {
    let res = common.client.list_contests().await;
    match res {
        Ok(data) => serde_json::to_value(data).unwrap(),
        Err(e) => {
            eprintln!("error: {}", e);
            Value::Null
        }
    }
}
