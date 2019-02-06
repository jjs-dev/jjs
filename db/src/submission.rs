use domain::Submission;
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
    pub fn create_submission(&'conn self, toolchain: &str) -> Submission {
        let query = "INSERT INTO submissions (toolchain, state) VALUES ($1,  'WaitInvoke') RETURNING submission_id";
        let res = self.conn.query(query, &[&toolchain]).expect("couldn't create submission in DB");
        let id_row = res.get(0);
        let s8n_id: i32 = id_row.get(0);
        Submission {
            id: s8n_id as u32,
            toolchain: toolchain.into(),
        }
    }

    pub fn find_by_id(&self, id: u32) -> Submission {
        let query = "SELECT toolchain FROM submissions WHERE submission_id = $1";
        let res = self.conn.query(query, &[&(id as i32)]).unwrap();
        let toolchain = res.get(0).get(0);
        Submission { id, toolchain }
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
                toolchain: row.get("toolchain"),
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
}
