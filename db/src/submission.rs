use postgres::GenericConnection;
use objects::Submission;

pub trait Submissions {
    fn create_submission(&self, toolchain: &str) -> Submission;
    fn find_by_id(&self, id: usize) -> Submission;
}

pub struct PgSubmissions {
    conn: Box<dyn postgres::GenericConnection>,
}

impl PgSubmissions {
    pub fn new(conn: Box<dyn GenericConnection>) -> PgSubmissions {
        PgSubmissions {
            conn,
        }
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
        let query = "SELECT toolchain FROM submissions WHERE id = $1";
        let res = self.conn.query(query, &[&(id as i32)]).unwrap();
        let toolchain = res.get(0).get(0);
        Submission {
            id,
            toolchain,
        }
    }
}