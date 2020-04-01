use async_trait::async_trait;
use log::{debug, warn};
use std::path::{Path, PathBuf};

pub struct ConnectionSettings {
    pub conn_string: String,
    pub db_name: String,
}

#[derive(Clone, Copy)]
pub struct DbContext<'a> {
    pub settings: &'a ConnectionSettings,
    pub install_dir: &'a Path,
}

impl<'a> DbContext<'a> {
    fn migrations_dir(self) -> PathBuf {
        self.install_dir.join("share/db")
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("db error")]
    Pg(#[from] tokio_postgres::Error),
    #[error("io error")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
enum DatabaseVersion {
    RevisionInfoMissing,
    Missing,
    Version {
        current: String,
        latest: Option<String>,
    },
}

pub struct Database<'a> {
    version: DatabaseVersion,
    cx: DbContext<'a>,
}

#[async_trait]
impl<'a> crate::Component for Database<'a> {
    type Error = Error;

    async fn state(&self) -> Result<crate::StateKind, Error> {
        let status = match &self.version {
            DatabaseVersion::RevisionInfoMissing => crate::StateKind::Errored,
            DatabaseVersion::Missing => crate::StateKind::Upgradable,
            DatabaseVersion::Version {
                current: _current,
                latest,
            } => {
                if latest.is_none() {
                    crate::StateKind::UpToDate
                } else {
                    crate::StateKind::Upgradable
                }
            }
        };
        Ok(status)
    }

    async fn upgrade(&self) -> Result<(), Error> {
        let current_version = match &self.version {
            DatabaseVersion::Version { current, .. } => Some(current.as_str()),
            DatabaseVersion::Missing => None,
            _ => panic!("called upgrade() for non-upgradable DB"),
        };
        let migrations = unapplied_migrations(self.cx, current_version).await?;
        let client = do_connect(self.cx.settings, Some(&self.cx.settings.db_name)).await?;
        for mig in migrations {
            let apply_script_path = self.cx.migrations_dir().join(mig).join("up.sql");
            let apply_script = tokio::fs::read_to_string(&apply_script_path).await?;
            client.simple_query(&apply_script).await?;
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "database"
    }
}

impl<'a> Database<'a> {}

impl<'a> std::fmt::Display for Database<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.version {
            DatabaseVersion::RevisionInfoMissing => {
                write!(f, "database exists, but does not contain version info")
            }
            DatabaseVersion::Missing => write!(f, "database does not exist"),
            DatabaseVersion::Version { current, latest } => {
                write!(f, "database version is {}", current)?;
                if let Some(latest) = latest {
                    write!(f, ", latest is {}", &latest)?;
                }
                Ok(())
            }
        }
    }
}

async fn unapplied_migrations(
    cx: DbContext<'_>,
    current_version: Option<&str>,
) -> Result<Vec<String>, Error> {
    let mut items = tokio::fs::read_dir(cx.install_dir.join("share/db")).await?;
    let mut names = std::collections::BTreeSet::new();
    while let Some(item) = items.next_entry().await? {
        let name = item.path();
        let name = name.file_stem().unwrap();
        names.insert(name.to_str().unwrap().to_string());
    }
    match current_version {
        Some(current_version) => Ok(names
            .into_iter()
            .filter(|migration| migration.as_str() > current_version)
            .collect()),
        None => Ok(names.into_iter().collect()),
    }
}

async fn last_version(
    cx: DbContext<'_>,
    current_version: Option<&str>,
) -> Result<Option<String>, Error> {
    unapplied_migrations(cx, current_version)
        .await
        .map(|migrations| migrations.into_iter().next_back())
}

async fn do_connect(
    settings: &ConnectionSettings,
    override_db_name: Option<&str>,
) -> Result<tokio_postgres::Client, Error> {
    let mut config: tokio_postgres::Config = settings.conn_string.parse()?;
    if let Some(name) = override_db_name {
        config.dbname(name);
    }
    let (client, connection) = config.connect(tokio_postgres::NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    Ok(client)
}

async fn get_db_version(cx: DbContext<'_>) -> Result<DatabaseVersion, Error> {
    let client_0 = do_connect(cx.settings, None).await?;

    let databases = client_0
        .query("SELECT datname FROM pg_database", &[])
        .await?;
    let mut found_db = false;
    for db in databases {
        let db_name: String = db.get("datname");
        if db_name == cx.settings.db_name {
            debug!("Found database");
            found_db = true;
        }
    }
    if !found_db {
        return Ok(DatabaseVersion::Missing);
    }
    let client = do_connect(&cx.settings, Some(cx.settings.db_name.as_str())).await?;

    let all_tables = client
        .query(
            "SELECT table_name FROM information_schema.tables WHERE table_schema='public'",
            &[],
        )
        .await?;
    if all_tables.is_empty() {
        debug!("Database exists, but it is empty; marking as Missing");
        return Ok(DatabaseVersion::Missing);
    }

    let revision = match client.query_one("SELECT * FROM __revision", &[]).await {
        Ok(rev) => rev,
        Err(err) => {
            if err.code() == Some(&tokio_postgres::error::SqlState::UNDEFINED_TABLE) {
                warn!("table __revision does not exist");
                return Ok(DatabaseVersion::RevisionInfoMissing);
            } else {
                return Err(err.into());
            }
        }
    };
    let revision: String = revision.get("revision");

    let last_version = last_version(cx, Some(revision.as_str())).await?;

    Ok(DatabaseVersion::Version {
        current: revision,
        latest: last_version,
    })
}

pub async fn analyze<'a>(cx: DbContext<'a>) -> Result<Database<'a>, Error> {
    let version = get_db_version(cx).await?;
    Ok(Database { cx, version })
}
