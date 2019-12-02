use super::{InvocationsRepo, Repo, RunsRepo, UsersRepo};
use crate::schema::*;
use anyhow::{bail, Result};
use std::{convert::TryFrom, sync::Mutex};

#[derive(Debug, Default)]
struct Data {
    // None if run was deleted
    runs: Vec<Option<Run>>,
    invs: Vec<Invocation>,
    users: Vec<User>,
}

#[derive(Debug, Default)]
pub struct MemoryRepo {
    conn: Mutex<Data>,
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
        .unwrap();
        this.user_new(NewUser {
            username: "Global/Guest".to_string(),
            password_hash: None,
            groups: vec![],
        })
        .unwrap();
        this
    }
}

impl RunsRepo for MemoryRepo {
    fn run_new(&self, run_data: NewRun) -> Result<Run> {
        let mut data = self.conn.lock().unwrap();
        let run_id = data.runs.len() as RunId;
        let run = Run {
            id: run_id,
            toolchain_id: run_data.toolchain_id,
            status_code: run_data.status_code,
            status_kind: run_data.status_kind,
            problem_id: run_data.problem_id,
            score: run_data.score,
            rejudge_id: run_data.rejudge_id,
            user_id: run_data.user_id,
        };
        data.runs.push(Some(run.clone()));
        Ok(run)
    }

    fn run_try_load(&self, run_id: i32) -> Result<Option<Run>> {
        let data = self.conn.lock().unwrap();
        let idx = run_id as usize;
        Ok(data.runs.get(idx).cloned().unwrap_or(None))
    }

    fn run_update(&self, run_id: i32, patch: RunPatch) -> Result<()> {
        let mut data = self.conn.lock().unwrap();
        let idx = run_id as usize;
        let cur = match data.runs.get_mut(idx) {
            Some(Some(x)) => x,
            None | Some(None) => bail!("run_update@memory: unknown run id"),
        };
        if let Some(new_status_code) = patch.status_code {
            cur.status_code = new_status_code;
        }
        if let Some(new_status_kind) = patch.status_kind {
            cur.status_kind = new_status_kind;
        }
        if let Some(new_score) = patch.score {
            cur.score = new_score;
        }
        if let Some(new_rejudge_id) = patch.rejudge_id {
            cur.rejudge_id = new_rejudge_id;
        }

        Ok(())
    }

    fn run_delete(&self, run_id: i32) -> Result<()> {
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

    fn run_select(&self, with_run_id: Option<RunId>, limit: Option<u32>) -> Result<Vec<Run>> {
        let lim = limit
            .map(|x| usize::try_from(x).unwrap())
            .unwrap_or(usize::max_value());
        if lim == 0 {
            return Ok(Vec::new());
        }
        match with_run_id {
            Some(r) => Ok(self
                .run_try_load(r)
                .into_iter()
                .filter_map(std::convert::identity)
                .collect()),
            None => {
                let data = self.conn.lock().unwrap();
                let cnt = std::cmp::min(lim, data.runs.len());
                Ok(data.runs[..cnt].iter().filter_map(|x| x.clone()).collect())
            }
        }
    }
}

impl InvocationsRepo for MemoryRepo {
    fn inv_new(&self, inv_data: NewInvocation) -> Result<Invocation> {
        let mut data = self.conn.lock().unwrap();
        let inv_id = data.invs.len() as InvocationId;
        let inv = Invocation {
            id: inv_id,
            invoke_task: inv_data.invoke_task,
        };
        data.invs.push(inv.clone());
        Ok(inv)
    }

    fn inv_pop(&self) -> Result<Option<Invocation>> {
        let mut data = self.conn.lock().unwrap();
        Ok(data.invs.pop())
    }
}

impl UsersRepo for MemoryRepo {
    fn user_new(&self, user_data: NewUser) -> Result<User> {
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

    fn user_try_load_by_login(&self, login: &str) -> Result<Option<User>> {
        let data = self.conn.lock().unwrap();
        let res = data
            .users
            .iter()
            .find(|user| user.username == login)
            .cloned();
        Ok(res)
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
            assert!(repo.run_load(228).is_err());
            assert!(repo.run_load(0).is_err());
            let new_run = NewRun {
                toolchain_id: "foo".to_string(),
                status_code: "bar".to_string(),
                status_kind: "baz".to_string(),
                problem_id: "quux".to_string(),
                score: 444,
                rejudge_id: 33,
                user_id: john_id,
            };
            let inserted_run = repo.run_new(new_run).unwrap();
            assert_eq!(inserted_run.id, 0);
            let run_in_db = repo.run_load(0).unwrap();
            assert_eq!(inserted_run, run_in_db);
        }

        #[test]
        fn test_patch() {
            let repo = MemoryRepo::new();
            let new_run = NewRun {
                toolchain_id: "0".to_string(),
                status_code: "0".to_string(),
                status_kind: "0".to_string(),
                problem_id: "0".to_string(),
                score: 0,
                rejudge_id: 0,
                user_id: uuid::Uuid::new_v4(),
            };
            repo.run_new(new_run).unwrap();
            let patch = RunPatch {
                status_code: Some("1".to_string()),
                status_kind: Some("2".to_string()),
                score: Some(3),
                rejudge_id: Some(4),
            };
            repo.run_update(0, patch).unwrap();
            let patched_run = repo.run_load(0).unwrap();
            // now let's check that all fields that must be updated are actually updated
            assert_eq!(patched_run.status_code, "1");
            assert_eq!(patched_run.status_kind, "2");
            assert_eq!(patched_run.score, 3);
            assert_eq!(patched_run.rejudge_id, 4);
        }
    }
}
