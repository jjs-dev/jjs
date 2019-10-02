use crate::CommonParams;
use graphql_client::GraphQLQuery;
use serde_json::Value;

pub fn exec(cx: &CommonParams) -> Value {
    let vars = crate::queries::api_version::Variables {};

    let res = cx
        .client
        .query::<_, crate::queries::api_version::ResponseData>(
            &crate::queries::ApiVersion::build_query(vars),
        )
        .expect("network error")
        .into_result();
    match res {
        Ok(data) => {
            let vers = data.api_version;
            let items = vers.split('.').collect::<Vec<_>>();
            assert_eq!(items.len(), 2);
            serde_json::json!({
                "major": items[0],
                "minor": items[1]
            })
        }
        Err(e) => {
            eprintln!("Error: {}", e[0]);
            Value::Null
        }
    }
}
