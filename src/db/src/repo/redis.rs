use super::KvRepo;
use anyhow::{Context as _, Result};
use redis::AsyncCommands as _;
use std::sync::Arc;

#[derive(Clone)]
pub struct RedisRepo {
    conn: Arc<tokio::sync::Mutex<redis::aio::Connection>>,
}

impl std::fmt::Debug for RedisRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("RedisRepo").field("conn", &"..").finish()
    }
}

impl RedisRepo {
    pub(crate) async fn new(conn_url: &str) -> Result<RedisRepo> {
        let client = redis::Client::open(conn_url).context("invalid connection string")?;
        let conn = client
            .get_async_connection()
            .await
            .context("unable to connect")?;
        let conn = Arc::new(tokio::sync::Mutex::new(conn));
        Ok(RedisRepo { conn })
    }
}

#[async_trait::async_trait]
impl KvRepo for RedisRepo {
    async fn kv_get_raw(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.conn.lock().await.get(key).await.map_err(Into::into)
    }

    async fn kv_put_raw(&self, key: &str, value: &[u8]) -> Result<()> {
        self.conn
            .lock()
            .await
            .set(key, value)
            .await
            .map_err(Into::into)
    }

    async fn kv_del(&self, key: &str) -> Result<()> {
        self.conn.lock().await.del(key).await.map_err(Into::into)
    }
}
