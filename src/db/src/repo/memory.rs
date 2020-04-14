use super::{InvocationsRepo, KvRepo, ParticipationsRepo, Repo, RunsRepo, UsersRepo};
use crate::schema::*;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use futures::future::FutureExt;
use std::{
    convert::TryFrom,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
struct Data {
    // None if run was deleted
    runs: Vec<Option<Run>>,
    invs: Vec<Invocation>,
    users: Vec<User>,
    kv: std::collections::HashMap<String, Vec<u8>>,
    parts: Vec<Participation>,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryRepo {
    conn: Arc<Mutex<Data>>,
}

impl MemoryRepo {
    pub fn new() -> Self {
        // TODO duplicates db/migrations/<initial>/up.sql
        let this: Self = Self::default();
        this.user_new(NewUser {
            username: "Global/Root".to_string(),
            password_hash: None,
            groups: vec![],
        })
        .now_or_never()
        .unwrap()
        .unwrap();
        this.user_new(NewUser {
            username: "Global/Guest".to_string(),
            password_hash: None,
            groups: vec![],
        })
        .now_or_never()
        .unwrap()
        .unwrap();
        this
    }
}

#[async_trait]
impl RunsRepo for MemoryRepo {
    async fn run_new(&self, run_data: NewRun) -> Result<Run> {
        let mut data = self.conn.lock().unwrap();
        let run_id = data.runs.len() as RunId;
        let run = Run {
            id: run_id,
            toolchain_id: run_data.toolchain_id,
            problem_id: run_data.problem_id,
            rejudge_id: run_data.rejudge_id,
            user_id: run_data.user_id,
            contest_id: run_data.contest_id,
        };
        data.runs.push(Some(run.clone()));
        Ok(run)
    }

    async fn run_try_load(&self, run_id: i32) -> Result<Option<Run>> {
        let data = self.conn.lock().unwrap();
        let idx = run_id as usize;
        Ok(data.runs.get(idx).cloned().unwrap_or(None))
    }

    async fn run_update(&self, run_id: i32, patch: RunPatch) -> Result<()> {
        let mut data = self.conn.lock().unwrap();
        let idx = run_id as usize;
        let cur = match data.runs.get_mut(idx) {
            Some(Some(x)) => x,
            None | Some(None) => bail!("run_update@memory: unknown run id"),
        };
        if let Some(new_rejudge_id) = patch.rejudge_id {
            cur.rejudge_id = new_rejudge_id;
        }

        Ok(())
    }

    async fn run_delete(&self, run_id: i32) -> Result<()> {
        let mut data = self.conn.lock().unwrap();
        let cur = match data.runs.get_mut(run_id as usize) {
            Some(x) => x,
            None => bail!("run_delete@memory: unknown run id"),
        };
        if cur.take().is_some() {
            Ok(())
        } else {
            bail!("run_delete@memory: run already deleted")
        }
    }

    async fn run_select(&self, user_id: Option<UserId>, limit: Option<u32>) -> Result<Vec<Run>> {
        let lim = limit
            .map(|x| usize::try_from(x).unwrap())
            .unwrap_or(usize::max_value());
        if lim == 0 {
            return Ok(Vec::new());
        }

        let data = self.conn.lock().unwrap();
        let cnt = std::cmp::min(lim, data.runs.len());
        Ok(data
            .runs
            .iter()
            .filter_map(|x| x.clone())
            .filter(|run| match user_id {
                Some(user_id) => user_id == run.user_id,
                None => true,
            })
            .take(cnt)
            .collect())
    }
}

#[async_trait]
impl InvocationsRepo for MemoryRepo {
    async fn inv_new(&self, inv_data: NewInvocation) -> Result<Invocation> {
        let mut data = self.conn.lock().unwrap();
        let inv_id = data.invs.len() as InvocationId;
        let inv = Invocation {
            id: inv_id,
            run_id: inv_data.run_id,
            invoke_task: inv_data.invoke_task,
            state: inv_data.state,
            outcome: inv_data.outcome,
        };
        data.invs.push(inv.clone());
        Ok(inv)
    }

    async fn inv_find_waiting(
        &self,
        offset: u32,
        count: u32,
        predicate: &mut (dyn FnMut(Invocation) -> Result<bool> + Send + Sync),
    ) -> Result<Vec<Invocation>> {
        let data = self.conn.lock().unwrap();
        let items = data.invs.iter().skip(offset as usize).take(count as usize);
        let mut filtered = Vec::new();
        for x in items {
            if predicate(x.clone())? {
                filtered.push(x.clone());
            }
        }
        Ok(filtered)
    }

    async fn inv_last(&self, run_id: RunId) -> Result<Invocation> {
        let data = self.conn.lock().unwrap();
        data.invs
            .iter()
            .filter(|inv| inv.run_id == run_id)
            .last()
            .ok_or_else(|| anyhow::anyhow!("no invocations for run exist"))
            .map(Clone::clone)
    }

    async fn inv_update(&self, inv_id: InvocationId, patch: InvocationPatch) -> Result<()> {
        let mut data = self.conn.lock().unwrap();
        if inv_id >= data.invs.len() as i32 || inv_id < 0 {
            bail!("inv_update: no such invocation");
        }
        let mut inv = &mut data.invs[inv_id as usize];
        let InvocationPatch { state: p_state } = patch;
        if let Some(p_state) = p_state {
            inv.state = p_state;
        }
        Ok(())
    }

    async fn inv_add_outcome_header(
        &self,
        inv_id: InvocationId,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> Result<()> {
        let mut data = self.conn.lock().unwrap();
        let inv = match data.invs.get_mut(inv_id as usize) {
            Some(inv) => inv,
            None => bail!("inv_add_outcome_header: no such invocation"),
        };
        let headers = match inv.outcome.as_array_mut() {
            Some(hs) => hs,
            None => bail!("inb_add_outcome_header: outcome is not array"),
        };
        headers.push(
            serde_json::to_value(&header).context("failed to serialize InvokeOutcomeHeader")?,
        );
        Ok(())
    }
}

#[async_trait]
impl UsersRepo for MemoryRepo {
    async fn user_new(&self, user_data: NewUser) -> Result<User> {
        let mut data = self.conn.lock().unwrap();
        let user_id = data.users.len();
        let user_id = uuid::Uuid::from_fields(user_id as u32, 0, 0, &[0; 8]).unwrap();
        let user = User {
            id: user_id,
            username: user_data.username,
            password_hash: user_data.password_hash,
            groups: user_data.groups,
        };
        data.users.push(user.clone());
        Ok(user)
    }

    async fn user_try_load_by_login(&self, login: &str) -> Result<Option<User>> {
        let data = self.conn.lock().unwrap();
        let res = data
            .users
            .iter()
            .find(|user| user.username == login)
            .cloned();
        Ok(res)
    }
}

#[async_trait]
impl KvRepo for MemoryRepo {
    async fn kv_get_raw(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let data = self.conn.lock().unwrap();
        Ok(data.kv.get(key).map(ToOwned::to_owned))
    }

    async fn kv_put_raw(&self, key: &str, value: &[u8]) -> Result<()> {
        let mut data = self.conn.lock().unwrap();
        data.kv.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    async fn kv_del(&self, key: &str) -> Result<()> {
        let mut data = self.conn.lock().unwrap();
        data.kv.remove(key);
        Ok(())
    }
}

#[async_trait]
impl ParticipationsRepo for MemoryRepo {
    async fn part_new(&self, part_data: NewParticipation) -> Result<Participation> {
        let mut data = self.conn.lock().unwrap();
        let part_id = data.parts.len() as ParticipationId;
        let part = Participation {
            id: part_id,
            user_id: part_data.user_id,
            contest_id: part_data.contest_id,
            phase: part_data.phase,
            virtual_contest_start_time: part_data.virtual_contest_start_time,
        };
        data.parts.push(part.clone());
        Ok(part)
    }

    async fn part_find(&self, id: ParticipationId) -> Result<Option<Participation>> {
        let data = self.conn.lock().unwrap();
        Ok(data.parts.get(id as usize).cloned())
    }

    async fn part_lookup(
        &self,
        user_id: UserId,
        contest_id: &str,
    ) -> Result<Option<Participation>> {
        let data = self.conn.lock().unwrap();
        Ok(data
            .parts
            .iter()
            .find(|item| item.contest_id == *contest_id && item.user_id == user_id)
            .cloned())
    }
}
impl Repo for MemoryRepo {}

#[cfg(test)]
mod tests {
    use super::*;

    mod runs {
        use super::*;

        #[test]
        fn test_basic() {
            let repo = MemoryRepo::new();

            let john_id = uuid::Uuid::new_v4();
            assert!(repo.run_load(228).now_or_never().unwrap().is_err());
            assert!(repo.run_load(0).now_or_never().unwrap().is_err());
            let new_run = NewRun {
                toolchain_id: "foo".to_string(),
                problem_id: "quux".to_string(),
                rejudge_id: 33,
                user_id: john_id,
                contest_id: "olymp".to_string(),
            };
            let inserted_run = repo.run_new(new_run).now_or_never().unwrap().unwrap();
            assert_eq!(inserted_run.id, 0);
            let run_in_db = repo.run_load(0).now_or_never().unwrap().unwrap();
            assert_eq!(inserted_run, run_in_db);
        }

        #[test]
        fn test_patch() {
            let repo = MemoryRepo::new();
            let new_run = NewRun {
                toolchain_id: "0".to_string(),
                problem_id: "0".to_string(),
                rejudge_id: 0,
                user_id: uuid::Uuid::new_v4(),
                contest_id: "cntst".to_string(),
            };
            repo.run_new(new_run).now_or_never().unwrap().unwrap();
            let patch = RunPatch {
                rejudge_id: Some(4),
            };
            repo.run_update(0, patch).now_or_never().unwrap().unwrap();
            let patched_run = repo.run_load(0).now_or_never().unwrap().unwrap();
            // now let's check that all fields that must be updated are actually updated
            assert_eq!(patched_run.rejudge_id, 4);
        }
    }
}
