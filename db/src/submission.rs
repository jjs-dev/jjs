use postgres::GenericConnection;

pub trait Submissions {
    fn create_submission(&mut self, toolchain: &str, digest: &str);
}

pub struct PgSubmissions {
    conn: Box<dyn postgres::GenericConnection>,
}

impl PgSubmissions {
    fn new(conn: Box<dyn postgres::GenericConnection>) -> PgSubmissions {
        PgSubmissions {
            conn,
        }
    }
}

impl Submissions for PgSubmissions {
    fn create_submission(&mut self, toolchain: &str, digest: &str) {
        let query = "INSERT INTO submissions (toolchain, digest) VALUES ($1, $2);";
        let res = self.conn.query(query, &[&toolchain, &digest]).unwrap();
        println!("{:?}", res);
        unimplemented!()
    }
}