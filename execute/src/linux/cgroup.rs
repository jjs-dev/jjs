use std::path::PathBuf;

pub(crate) struct Hierarchy {
    base_path: PathBuf,
    prefix: PathBuf,
}

pub(crate) struct Cgroup {
    pub(crate) id: String,
}

impl Hierarchy {
    pub(crate) fn setup() -> Hierarchy {
        let base_path = PathBuf::from_str("/sys/fs/cgroup/").unwrap();
        let prefix = PathBuf::from_str("jjs/").unwrap();
        Hierarchy {
            base_path,
            prefix,
        }
    }
}