use client::prelude::Sendable;

pub async fn exec(api: &client::ApiClient) -> anyhow::Result<()> {
    let vers = client::models::ApiVersion::api_version().send(api).await?;
    println!("JJS API version: {}.{}", vers.major, vers.minor);
    Ok(())
}
