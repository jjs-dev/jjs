use std::path::{Path, PathBuf};

pub struct FileSubmissionContent {
    pub path: PathBuf,
}

pub enum SubmissionContent {
    File(FileSubmissionContent),
}

pub struct Submission {
    pub content: SubmissionContent,
    pub toolchain_name: String,
}

impl Submission {
    pub fn from_file_path(p: &Path, tc_name: &str) -> Submission {
        Submission {
            content: SubmissionContent::File(FileSubmissionContent {
                path: PathBuf::from(p),
            }),
            toolchain_name: String::from(tc_name),
        }
    }

    /*pub fn get_file_path(&self) -> Option<&Path> {
        match self.content {
            SubmissionContent::File(ref fsc) => Some(&(fsc.path)),
            //_ => None
        }
    }*/
}
