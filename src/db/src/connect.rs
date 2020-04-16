use crate::{
    repo::{MemoryRepo, PgRepo, RedisRepo},
    DbConn,
};
use anyhow::{Context, Result};
use futures::future::FutureExt;
use std::env;

pub struct ConnectOptions {
    /// Postgres connection string
    pg: Option<String>,
    /// Redis connection string
    redis: Option<String>,
}

impl ConnectOptions {
    fn warn(&self) {
        if cfg!(not(test)) && self.pg.is_none() {
            eprintln!(
                "warning: pg url not provided in DATABASE_URL. \
                 JJS is unusable in such configuration."
            );
        }
    }
}

pub async fn connect(options: ConnectOptions) -> Result<DbConn> {
    let mem = MemoryRepo::new();
    let pg = match options.pg {
        Some(pg_conn_str) => {
            let conn = PgRepo::new(&pg_conn_str)
                .await
                .context("cannot connect to postgres")?;
            Some(conn)
        }
        None => None,
    };
    let redis = match options.redis {
        Some(redis_conn_str) => {
            let conn = RedisRepo::new(&redis_conn_str)
                .await
                .context("cannot connect to redis")?;
            Some(conn)
        }
        None => None,
    };
    Ok(DbConn { mem, pg, redis })
}

pub async fn connect_env() -> Result<crate::DbConn> {
    let opts = ConnectOptions {
        pg: env::var("DATABASE_URL").ok(),
        redis: env::var("REDIS_URL").ok(),
    };
    opts.warn();
    connect(opts).await
}

pub fn connect_memory() -> Result<crate::DbConn> {
    let opts = ConnectOptions {
        pg: None,
        redis: None,
    };
    connect(opts).now_or_never().unwrap()
}
