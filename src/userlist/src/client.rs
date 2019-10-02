use graphql_client::GraphQLQuery;
use uuid::Uuid;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../frontend-api/src/schema-gen.json",
    query_path = "./src/create_user.graphql",
    response_derives = "Debug"
)]
pub struct CreateUser;
