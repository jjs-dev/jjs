use super::{InvocationRequestsRepo, Repo, RunsRepo, UsersRepo};
use crate::{schema::*, Error};
use std::sync::Mutex;

#[derive(Default)]
struct Data {
    runs: Vec<Run>,
    inv_reqs: Vec<InvocationRequest>,
    users: Vec<User>,
}

#[derive(Default)]
pub struct MemoryRepo {
    conn: Mutex<Data>,
}

impl MemoryRepo {
    pub fn new() -> Self {
        Default::default()
    }
}

impl RunsRepo for MemoryRepo {
    fn run_new(&self, run_data: NewRun) -> Result<Run, Error> {
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
        };
        data.runs.push(run.clone());
        Ok(run)
    }

    fn run_load(&self, run_id: i32) -> Result<Run, Error> {
        let data = self.conn.lock().unwrap();
        let idx = run_id as usize;
        data.runs
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::string("run_load: unknown run id"))
    }

    fn run_update(&self, run_id: i32, patch: RunPatch) -> Result<(), Error> {
        let mut data = self.conn.lock().unwrap();
        let idx = run_id as usize;
        let cur = match data.runs.get_mut(idx) {
            Some(x) => x,
            None => return Err(Error::string("run_update: unknown run id")),
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
}

impl InvocationRequestsRepo for MemoryRepo {
    fn inv_req_new(&self, inv_req_data: NewInvocationRequest) -> Result<InvocationRequest, Error> {
        let mut data = self.conn.lock().unwrap();
        let inv_req_id = data.inv_reqs.len() as InvocationRequestId;
        let inv_req = InvocationRequest {
            id: inv_req_id,
            run_id: inv_req_data.run_id,
            invoke_revision: inv_req_data.invoke_revision,
        };
        data.inv_reqs.push(inv_req.clone());
        Ok(inv_req)
    }

    fn inv_req_pop(&self) -> Result<Option<InvocationRequest>, Error> {
        let mut data = self.conn.lock().unwrap();
        Ok(data.inv_reqs.pop())
    }
}

impl UsersRepo for MemoryRepo {
    fn user_new(&self, user_data: NewUser) -> Result<User, Error> {
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
            assert!(repo.run_load(228).is_err());
            assert!(repo.run_load(0).is_err());
            let new_run = NewRun {
                toolchain_id: "foo".to_string(),
                status_code: "bar".to_string(),
                status_kind: "baz".to_string(),
                problem_id: "quux".to_string(),
                score: 444,
                rejudge_id: 33,
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
