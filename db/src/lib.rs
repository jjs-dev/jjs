pub mod submission;

pub struct Db {
    pub submissions: Box<dyn submission::Submissions>,
}
