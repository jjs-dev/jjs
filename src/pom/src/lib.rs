use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileRefRoot {
    Problem,
    System,
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
