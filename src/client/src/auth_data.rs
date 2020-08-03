//! Defines AuthData - common connection&authentication config format
use anyhow::Context as _;
use std::path::PathBuf;
use tracing::instrument;
//use tracing::

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AuthData {
    /// API base url
    pub endpoint: String,
    /// Authentication options
    pub auth: AuthKind,
}

impl AuthData {
    pub(crate) fn validate(&self) -> Vec<anyhow::Error> {
        let mut errs = Vec::new();
        match url::Url::parse(&self.endpoint) {
            Ok(u) => {
                if u.scheme() != "http" && u.scheme() != "https" {
                    errs.push(anyhow::format_err!(
                        "endpoint: only http and https schemes are allowed, got {}",
                        u.scheme()
                    ));
                }
                if !u.path().ends_with('/') {
                    errs.push(anyhow::Error::msg(
                        "endpoint: url pathname must be nonempty and have trailing /",
                    ));
                }
            }
            Err(url_parse_err) => errs.push(
                anyhow::Error::new(url_parse_err).context("endpoint: field is not valid url"),
            ),
        }

        errs
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AuthKind {
    LoginAndPassword(AuthByLoginAndPassword),
    Token(AuthByToken),
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AuthByLoginAndPassword {
    pub login: String,
    pub password: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AuthByToken {
    pub token: String,
}

#[instrument]
fn get_path() -> anyhow::Result<PathBuf> {
    if let Some(p) = std::env::var_os("JJS_AUTH_DATA") {
        tracing::info!(path=%p.to_string_lossy(), "using path in JJS_AUTH_DATA");
        return Ok(p.into());
    }

    let dirs = xdg::BaseDirectories::with_prefix("jjs").context("XDG initialization failed")?;
    let path = dirs
        .find_config_file("authdata.yaml")
        .context("authdata.yaml file does not exist")?;
    tracing::info!(path=%path.display(), "resolved auth data via XDG");
    Ok(path)
}
impl AuthData {
    pub fn parse(data: &[u8]) -> anyhow::Result<AuthData> {
        let auth_data: AuthData = serde_yaml::from_slice(data).context("parse error")?;
        let errs = auth_data.validate();
        if !errs.is_empty() {
            let message = errs
                .into_iter()
                .map(|err| format!("{:#}\n", err))
                .collect::<Vec<_>>();
            let message = message.concat();
            anyhow::bail!("AuthData is invalid: {}", message);
        }
        Ok(auth_data)
    }

    #[instrument]
    pub async fn infer() -> anyhow::Result<AuthData> {
        let auth_data = match std::env::var("JJS_AUTH_DATA_INLINE") {
            Ok(ad) => {
                tracing::info!("found AuthData in JJS_AUTH_DATA_INLINE environment variable");
                ad.into_bytes()
            }
            Err(_) => {
                let auth_data_path = tokio::task::spawn_blocking(get_path)
                    .await
                    .unwrap()
                    .context("can not infer authdata.yaml path")?;
                tokio::fs::read(&auth_data_path).await.with_context(|| {
                    format!("failed to read auth data from {}", auth_data_path.display())
                })?
            }
        };

        Self::parse(&auth_data)
    }
}
