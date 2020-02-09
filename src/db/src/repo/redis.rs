use super::KvRepo;
use anyhow::{Context as _, Result};
use redis::AsyncCommands as _;

fn check_send<T: Send>(_: T) {}
fn check_sync<T: Sync>(_: T) {}

fn check_conn() {
    let conn: redis::aio::Connection = panic!();
    check_send(conn);
    let conn: redis::aio::Connection = panic!();
    check_sync(conn);
}

pub struct RedisRepo {
    conn: redis::aio::Connection,
}

fn check_redis_repo() {
    let rp: RedisRepo = panic!();
    check_send(rp);
    let rp: RedisRepo = panic!();
    check_sync(rp);
}

impl RedisRepo {
    pub(crate) async fn new(conn_url: &str) -> Result<RedisRepo> {
        let client = redis::Client::open(conn_url).context("invalid connection string")?;
        let conn = client
            .get_async_connection()
            .await
            .context("unable to connect")?;
        Ok(RedisRepo { conn })
    }
}

#[async_trait::async_trait]
impl KvRepo for RedisRepo {
    async fn kv_get_raw(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.conn.get(key).await.map_err(Into::into)
    }

    async fn kv_put_raw(&self, key: &str, value: &[u8]) -> Result<()> {
        self.conn.set(key, value).await.map_err(Into::into)
    }
}
