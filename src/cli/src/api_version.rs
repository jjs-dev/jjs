use crate::CommonParams;
use client::Api as _;
use serde_json::Value;

pub async fn exec(cx: &CommonParams) -> Value {
    let res = cx.client.api_version().await;
    match res {
        Ok(vers) => serde_json::json!({
            "major": vers.major,
            "minor": vers.minor
        }),
        Err(e) => {
            eprintln!("Error: {}", e);
            Value::Null
        }
    }
}
