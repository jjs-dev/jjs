pub mod submission;

pub struct Db<'conn> {
    pub submissions: submission::Submissions<'conn>,
}

impl<'c> Db<'c> {
    pub fn new(conn: &'c dyn postgres::GenericConnection) -> Self{
        Self {
            submissions: submission::Submissions::new(conn)
        }
    }
}