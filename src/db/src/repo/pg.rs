use super::{InvocationsRepo, KvRepo, ParticipationsRepo, Repo, RunsRepo, UsersRepo};
use crate::schema::*;
use anyhow::{Context, Result};
use bb8::{Pool, PooledConnection};

type ConnectionManager = bb8_postgres::PostgresConnectionManager<tokio_postgres::tls::NoTls>;

#[derive(Debug, Clone)]
pub struct PgRepo {
    pool: Pool<ConnectionManager>,
}

impl PgRepo {
    async fn conn(&self) -> Result<PooledConnection<'_, ConnectionManager>> {
        self.pool
            .get()
            .await
            .context("cannot obtain postgres connection")
    }

    pub(crate) async fn new(conn_url: &str) -> Result<PgRepo> {
        let conn_manager =
            ConnectionManager::new_from_stringlike(conn_url, tokio_postgres::tls::NoTls)?;
        let mut pool_builder = Pool::builder();
        // TODO refactor
        if let Some(timeout) = std::env::var("JJS_DB_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            let dur = std::time::Duration::from_secs(timeout);
            pool_builder = pool_builder.connection_timeout(dur);
        }
        let pool = pool_builder.build(conn_manager).await?;
        Ok(PgRepo { pool })
    }
}

#[async_trait::async_trait]
impl UsersRepo for PgRepo {
    async fn user_new(&self, user_data: NewUser) -> Result<User> {
        let user = User {
            id: uuid::Uuid::new_v4(),
            username: user_data.username,
            password_hash: user_data.password_hash,
            groups: user_data.groups,
        };
        self.conn()
            .await?
            .execute(
                "INSERT INTO users (id, username, password_hash, groups) VALUES ($1, $2, $3, $4)",
                &[&user.id, &user.username, &user.password_hash, &user.groups],
            )
            .await?;
        Ok(user)
    }

    async fn user_try_load_by_login(&self, login: &str) -> Result<Option<User>> {
        let row = self
            .conn()
            .await?
            .query_opt(
                "
            SELECT * FROM users WHERE username = $1
        ",
                &[&login],
            )
            .await?;

        match row {
            Some(row) => Ok(Some(User::from_pg_row(row))),
            None => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl InvocationsRepo for PgRepo {
    async fn inv_new(&self, inv_data: NewInvocation) -> Result<Invocation> {
        let id = self.conn().await?.query_one(
            "INSERT INTO invocations (run_id, state, invoke_task, outcome) VALUES ($1, $2, $3, $4) RETURNING id",
            &[
                &inv_data.run_id,
                &inv_data.state,
                &inv_data.invoke_task,
                &inv_data.outcome,
            ],
        ).await?.get(0);
        Ok(Invocation {
            id,
            run_id: inv_data.run_id,
            invoke_task: inv_data.invoke_task,
            state: inv_data.state,
            outcome: inv_data.outcome,
        })
    }

    async fn inv_last(&self, run_id: RunId) -> Result<Invocation> {
        let row = self
            .conn()
            .await?
            .query_one(
                "SELECT invocations.* FROM invocations
       INNER JOIN runs
       ON
          invocations.run_id = runs.id
       WHERE runs.id = $1
       ORDER BY invocations.id DESC
       LIMIT 1
       ",
                &[&run_id],
            )
            .await?;
        Ok(super::Invocation::from_pg_row(row))
    }

    async fn inv_find_waiting(
        &self,
        offset: u32,
        count: u32,
        predicate: &mut (dyn FnMut(Invocation) -> Result<bool> + Send + Sync),
    ) -> Result<Vec<Invocation>> {
        let offset = offset as i64;
        let count = count as i64;
        let mut conn = self.conn().await?;
        let tran = conn.transaction().await?;
        let query = "
    SELECT * FROM invocations
    WHERE state = $3
    ORDER BY id
    OFFSET $1 LIMIT $2
    FOR UPDATE
                ";
        let rows = tran
            .query(query, &[&offset, &count, &InvocationState::Queue.as_int()])
            .await
            .context("failed to load invocations slice")?;
        let invs: Vec<Invocation> = rows.into_iter().map(Invocation::from_pg_row).collect();
        let mut filtered = Vec::new();
        let mut to_del = Vec::new();
        for inv in invs {
            let inv_id = inv.id;
            let passed = predicate(inv.clone())
                .with_context(|| format!("predicate failed on invocation with id={}", inv_id))?;
            if passed {
                filtered.push(inv);
                to_del.push(inv_id);
            }
        }
        const STATE: i16 = InvocationState::InWork.as_int();
        tran.execute(
            "UPDATE invocations SET state = $1 WHERE id = ANY($2)",
            &[&STATE, &to_del],
        )
        .await
        .context("failed to mark invocations as running")?;
        tran.commit().await.context("transaction commit error")?;
        Ok(filtered)
    }

    async fn inv_update(&self, inv_id: InvocationId, patch: InvocationPatch) -> Result<()> {
        self.conn()
            .await?
            .execute(
                "UPDATE invocations SET
                state = COALESCE($1, state) 
        WHERE id = $2
                ",
                &[&patch.state, &inv_id],
            )
            .await?;
        Ok(())
    }

    async fn inv_add_outcome_header(
        &self,
        inv_id: InvocationId,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> Result<()> {
        let query = "
            UPDATE invocations SET
            outcome = outcome || $1
            WHERE id = $2
            ";
        let header =
            serde_json::to_value(&header).context("failed to serialize InvokeOutcomeHeader")?;
        self.conn()
            .await?
            .execute(query, &[&header, &inv_id])
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl RunsRepo for PgRepo {
    async fn run_new(&self, run_data: NewRun) -> Result<Run> {
        let row = self.conn().await?.query_one(
            "INSERT INTO runs (contest_id, problem_id, rejudge_id, toolchain_id, user_id) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            &[
                &run_data.contest_id,
                &run_data.problem_id,
                &run_data.rejudge_id,
                &run_data.toolchain_id,
                &run_data.user_id,
            ],
        ).await?;
        let id = row.get(0);
        Ok(Run {
            id,
            contest_id: run_data.contest_id,
            problem_id: run_data.problem_id,
            rejudge_id: run_data.rejudge_id,
            toolchain_id: run_data.toolchain_id,
            user_id: run_data.user_id,
        })
    }

    async fn run_try_load(&self, run_id: i32) -> Result<Option<Run>> {
        let row = self
            .conn()
            .await?
            .query_opt("SELECT * FROM runs WHERE id = $1", &[&run_id])
            .await?;
        Ok(row.map(Run::from_pg_row))
    }

    async fn run_update(&self, run_id: RunId, patch: RunPatch) -> Result<()> {
        self.conn()
            .await?
            .execute(
                "
        UPDATE runs SET
            rejudge_id = COALESCE($1, rejudge_id)
        WHERE id = $2
        ",
                &[&patch.rejudge_id, &run_id],
            )
            .await?;
        Ok(())
    }

    async fn run_delete(&self, run_id: RunId) -> Result<()> {
        self.conn()
            .await?
            .execute("DELETE FROM runs WHERE id = $1", &[&run_id])
            .await?;
        Ok(())
    }

    async fn run_select(&self, user_id: Option<UserId>, limit: Option<u32>) -> Result<Vec<Run>> {
        let limit = limit.map(|x| x as i32).unwrap_or(i32::max_value());
        let rows = self
            .conn()
            .await?
            .query(
                "SELECT * FROM runs WHERE COALESCE(user_id = $1, TRUE) LIMIT $2",
                &[&user_id, &limit],
            )
            .await?;
        Ok(rows.into_iter().map(Run::from_pg_row).collect())
    }
}

#[async_trait::async_trait]
impl KvRepo for PgRepo {
    async fn kv_get_raw(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let maybe_row = self
            .conn()
            .await?
            .query_opt("SELECT value FROM kv WHERE name = $1", &[&key])
            .await?;
        Ok(match maybe_row {
            Some(r) => Some(r.get("value")),
            None => None,
        })
    }

    async fn kv_put_raw(&self, key: &str, val: &[u8]) -> Result<()> {
        self.conn()
            .await?
            .execute(
                "INSERT INTO kv (name, value) VALUES ($1, $2) ON CONFLICT (name) DO 
                UPDATE SET value = $2",
                &[&key, &val],
            )
            .await?;
        Ok(())
    }

    async fn kv_del(&self, key: &str) -> Result<()> {
        self.conn()
            .await?
            .execute("DELETE FROM kv WHERE name = $1", &[&key])
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ParticipationsRepo for PgRepo {
    async fn part_new(&self, part_data: NewParticipation) -> Result<Participation> {
        let id = self.conn().await?.query_one(
            "INSERT INTO participations (contest_id, user_id, phase, virtual_contest_start_time) VALUES () RETURNING id", &[&part_data.contest_id, 
        &part_data.user_id, &part_data.phase, &part_data.virtual_contest_start_time]).await?;
        Ok(Participation {
            id: id.get("id"),
            contest_id: part_data.contest_id,
            user_id: part_data.user_id,
            phase: part_data.phase,
            virtual_contest_start_time: part_data.virtual_contest_start_time,
        })
    }

    async fn part_find(&self, reg_id: ParticipationId) -> Result<Option<Participation>> {
        let row = self
            .conn()
            .await?
            .query_opt("SELECT * FROM participations WHERE id = $1", &[&reg_id])
            .await?;

        Ok(row.map(Participation::from_pg_row))
    }

    async fn part_lookup(&self, uid: UserId, cid: &str) -> Result<Option<Participation>> {
        let row = self
            .conn()
            .await?
            .query_opt(
                "SELECT * FROM participations WHERE user_id = $1 AND contest_id = $2",
                &[&uid, &cid],
            )
            .await?;
        Ok(row.map(Participation::from_pg_row))
    }
}

impl Repo for PgRepo {}
