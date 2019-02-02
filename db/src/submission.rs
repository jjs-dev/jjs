use domain::Submission;
use postgres::GenericConnection;

pub trait Submissions {
    fn create_submission(&self, toolchain: &str) -> Submission;
    fn find_by_id(&self, id: usize) -> Submission;
    fn find_next_waiting(&self) -> Option<Submission>;
    fn update_submission_state(&self, submission: &Submission, new_state: &str);
}

pub struct PgSubmissions {
    conn: Box<dyn postgres::GenericConnection>,
}

impl PgSubmissions {
    pub fn new(conn: Box<dyn GenericConnection>) -> PgSubmissions {
        PgSubmissions { conn }
    }
}

impl Submissions for PgSubmissions {
    fn create_submission(&self, toolchain: &str) -> Submission {
        let query = "INSERT INTO submissions (toolchain) VALUES ($1) RETURNING submission_id;";
        let res = self.conn.query(query, &[&toolchain]).unwrap();
        let id_row = res.get(0);
        let s8n_id: i32 = id_row.get(0);
        Submission {
            id: s8n_id as usize,
            toolchain: toolchain.into(),
        }
    }

    fn find_by_id(&self, id: usize) -> Submission {
        let query = "SELECT toolchain FROM submissions WHERE submission_id = $1";
        let res = self.conn.query(query, &[&(id as i32)]).unwrap();
        let toolchain = res.get(0).get(0);
        Submission { id, toolchain }
    }

    fn find_next_waiting(&self) -> Option<Submission> {
        let query = include_str!("../queries/find_next_waiting.sql");
        let res = self.conn.query(query, &[]).unwrap();
        if res.is_empty() {
            None
        } else {
            let row = res.get(0);
            let sub_id: i32 = row.get("submission_id");
            Some(Submission {
                id: sub_id as usize,
                toolchain: row.get("toolchain")
            })
        }
    }

    fn update_submission_state(&self, submission: &Submission, new_state: &str) {
        let query = include_str!("../queries/update_submission_state.sql");
        let _res = self.conn.execute(query, &[&(submission.id as i32), &new_state]).expect("update_submission_state query failed");
    }
}
