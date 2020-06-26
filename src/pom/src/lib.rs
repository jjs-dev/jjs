use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Limits {
    /// Memory limit in bytes
    pub memory: Option<u64>,
    /// Time limit in milliseconds
    pub time: Option<u64>,
    /// Process count limit
    pub process_count: Option<u64>,
    /// Working dir size limit in bytes
    pub work_dir_size: Option<u64>,
}

impl Limits {
    fn default_num_procs() -> u64 {
        16
    }

    fn default_memory() -> u64 {
        256 * 1024 * 1024
    }

    fn default_time() -> u64 {
        3000
    }

    fn default_work_dir_size() -> u64 {
        16 * 1024 * 1024
    }

    pub fn time(self) -> u64 {
        self.time.unwrap_or_else(Self::default_time)
    }

    pub fn memory(self) -> u64 {
        self.memory.unwrap_or_else(Self::default_memory)
    }

    pub fn process_count(self) -> u64 {
        self.process_count.unwrap_or_else(Self::default_num_procs)
    }

    pub fn work_dir_size(self) -> u64 {
        self.work_dir_size
            .unwrap_or_else(Self::default_work_dir_size)
    }
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            memory: Some(Limits::default_memory()),
            time: Some(Limits::default_time()),
            process_count: Some(Limits::default_num_procs()),
            work_dir_size: Some(Limits::default_work_dir_size()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileRefRoot {
    Problem,
    Root,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRef {
    pub root: FileRefRoot,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Test {
    pub path: FileRef,
    pub correct: Option<FileRef>,
    pub limits: Limits,
    pub group: String,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct TestId(pub std::num::NonZeroU32);

impl TestId {
    /// Creates `TestId` from `id`
    /// # Panics
    /// Will panic if `id` is 0. Only use this function when you can prove `id` is non-null.
    pub fn make(id: u32) -> Self {
        Self(std::num::NonZeroU32::new(id).expect("TestId must be non-null"))
    }

    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl From<TestId> for u32 {
    fn from(t: TestId) -> u32 {
        t.get()
    }
}

impl std::fmt::Display for TestId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("TestId").field(&self.0.get()).finish()
    }
}

impl TestId {
    pub fn to_idx(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

impl std::ops::Index<TestId> for Vec<Test> {
    type Output = Test;

    fn index(&self, index: TestId) -> &Self::Output {
        &self[index.to_idx()]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub title: String,
    pub name: String,
    pub tests: Vec<Test>,
    pub checker_exe: FileRef,
    pub checker_cmd: Vec<String>,
    pub valuer_exe: FileRef,
    pub valuer_cfg: FileRef,
}
