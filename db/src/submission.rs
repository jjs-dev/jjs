use domain::{Submission, SubmissionState};
use postgres::GenericConnection;

pub struct Submissions<'conn> {
    conn: &'conn dyn postgres::GenericConnection,
}

impl<'conn> Submissions<'conn> {
    pub fn new(conn: &'conn dyn GenericConnection) -> Submissions<'conn> {
        Submissions { conn }
    }
}

impl<'conn> Submissions<'conn> {
    pub fn create_submission(&self, toolchain: String) -> Submission {
        let query = "INSERT INTO submissions (toolchain_id, state) VALUES ($1,  'WaitInvoke') RETURNING submission_id";
        let res = self
            .conn
            .query(query, &[&toolchain])
            .expect("couldn't create submission in DB");
        let id_row = res.get(0);
        let s8n_id: i32 = id_row.get("submission_id");
        Submission {
            id: s8n_id as u32,
            toolchain,
            state: SubmissionState::WaitInvoke,
        }
    }

    pub fn find_by_id(&self, id: u32) -> Submission {
        let query = "SELECT toolchain_id, state FROM submissions WHERE submission_id = $1";
        let res = self.conn.query(query, &[&(id as i32)]).unwrap();
        let toolchain = res.get(0).get("toolchain");
        let state = res.get(0).get("state");
        Submission {
            id,
            toolchain,
            state,
        }
    }

    pub fn find_next_waiting(&self) -> Option<Submission> {
        let query = include_str!("../queries/find_next_waiting.sql");
        let res = self.conn.query(query, &[]).unwrap();
        if res.is_empty() {
            None
        } else {
            let row = res.get(0);
            let sub_id: i32 = row.get("submission_id");
            Some(Submission {
                id: sub_id as u32,
                toolchain: row.get("toolchain_id"),
                state: SubmissionState::WaitInvoke,
            })
        }
    }

    pub fn update_submission_state(
        &self,
        submission: &Submission,
        new_state: domain::SubmissionState,
    ) {
        let query = include_str!("../queries/update_submission_state.sql");
        let _res = self
            .conn
            .execute(query, &[&(submission.id as i32), &new_state])
            .expect("update_submission_state query failed");
    }

    pub fn get_all(&self, limit: u32) -> Vec<Submission> {
        let query = "SELECT submission_id, toolchain_id, state FROM submissions LIMIT $1";
        let res = self.conn.query(query, &[&limit]).expect("DB query failed");
        res.iter()
            .map(|r| Submission {
                id: r.get("submission_id"),
                toolchain: r.get("toolchain_id"),
                state: r.get("state"),
            })
            .collect()
    }
}
