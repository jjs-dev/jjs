use anyhow::Context as _;
use schemars::JsonSchema;
use serde::{de::Error as _, Deserialize, Serialize};
use std::{path::Path, sync::Arc};
#[derive(Copy, Clone, Debug, Serialize, JsonSchema)]
pub enum Env {
    Prod,
    Dev,
}

impl Env {
    pub fn is_dev(self) -> bool {
        use Env::*;
        match self {
            Dev => true,
            Prod => false,
        }
    }
}

pub fn derive_key_512(secret: &str) -> Vec<u8> {
    use digest::Digest;
    use rand::{Rng, SeedableRng};
    let secret_hash = {
        let mut hasher = sha3::Sha3_512::new();
        hasher.input(secret.as_bytes());
        let result = &hasher.result()[16..48];
        let mut out = [0 as u8; 32];
        out.copy_from_slice(&result);
        out
    };
    assert_eq!(secret_hash.len(), 32);
    let mut gen = rand_chacha::ChaChaRng::from_seed(secret_hash);
    let key_size = 32;
    let mut out = Vec::with_capacity(key_size);
    for _i in 0..key_size {
        out.push(gen.gen::<u8>());
    }

    out
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ListenConfig {
    #[serde(default = "default_listen_host")]
    pub host: String,
    #[serde(default = "default_listen_port")]
    pub port: u16,
}

impl Default for ListenConfig {
    fn default() -> Self {
        ListenConfig {
            host: default_listen_host(),
            port: default_listen_port(),
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for Env {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s: String = serde::de::Deserialize::deserialize(deserializer)?;
        match s.as_str() {
            "dev" | "devel" | "development" => Ok(Env::Dev),
            "prod" | "production" => Ok(Env::Prod),
            _ => Err(D::Error::custom("unknown environment")),
        }
    }
}

fn default_env() -> Env {
    if cfg!(debug_assertions) {
        Env::Dev
    } else {
        Env::Prod
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ApiserverConfig {
    #[serde(default)]
    pub listen: ListenConfig,
    #[serde(default = "default_unix_socket_path")]
    pub unix_socket_path: String,
    #[serde(default = "default_env")]
    pub env: Env,
    /// Public address of apiserver (must be visible to invoker)
    #[serde(default = "default_external_addr")]
    pub external_addr: Option<String>,
    #[serde(default)]
    pub tls: Option<TlsConfig>,
}

impl Default for ApiserverConfig {
    fn default() -> Self {
        Self {
            listen: ListenConfig::default(),
            unix_socket_path: default_unix_socket_path(),
            env: default_env(),
            external_addr: default_external_addr(),
            tls: None,
        }
    }
}

fn default_listen_port() -> u16 {
    1779
}

fn default_listen_host() -> String {
    "127.0.0.1".to_string()
}

fn default_unix_socket_path() -> String {
    "/tmp/jjs-auth-sock".to_string()
}

fn default_external_addr() -> Option<String> {
    Some("127.0.0.1".to_string())
}

impl ApiserverConfig {
    pub fn obtain(jjs_data_dir: &Path) -> anyhow::Result<ApiserverConfig> {
        let config_path = jjs_data_dir.join("etc/apiserver.yaml");
        if !config_path.exists() {
            anyhow::bail!("Apiserver config {} does not exist", config_path.display());
        }
        let config = std::fs::read(config_path).context("failed to read config")?;
        let config = serde_yaml::from_slice(&config).context("parse error")?;

        Ok(config)
    }

    pub fn into_apiserver_params(self) -> anyhow::Result<ApiserverParams> {
        let db_conn: Arc<dyn db::DbConn> =
            db::connect_env().context("db connection failed")?.into();

        let secret = std::env::var("JJS_SECRET_KEY").unwrap_or_else(|_| {
            if let Env::Dev = self.env {
                String::from("DEVEL_HARDCODED_TOKEN")
            } else {
                panic!("Error: running in production mode, but JJS_SECRET_KEY not specified");
            }
        });
        let secret = derive_key_512(&secret);
        let token_mgr = crate::TokenMgr::new(db_conn.clone(), secret.into());
        Ok(ApiserverParams {
            cfg: self,
            db_conn,
            token_mgr,
        })
    }
}

// TODO: needs refactoring. Probably should be deleted.
#[derive(Debug)]
pub struct ApiserverParams {
    pub token_mgr: crate::api::TokenMgr,
    pub db_conn: Arc<dyn db::DbConn>,
    pub cfg: ApiserverConfig,
}
