use std::path::{Path, PathBuf};

pub struct FileSubmissionContent {
    path: PathBuf,
}

pub enum SubmissionContent {
    File(FileSubmissionContent),
}

pub struct Submission {
    content: SubmissionContent,
}

impl Submission {
    fn from_file_path(p: &Path) -> Submission {
        Submission {
            content: SubmissionContent::File(
                FileSubmissionContent {
                    path: PathBuf::from(p),
                }
            )
        }
    }

    fn get_file_path(&self) -> Option<&Path> {
        match self.content {
            SubmissionContent::File(ref fsc) => Some(&(fsc.path)),
            _ => None
        }
    }
}