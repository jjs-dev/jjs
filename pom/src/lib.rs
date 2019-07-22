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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub title: String,
    pub name: String,
    pub tests: Vec<Test>,
    pub checker_exe: FileRef,
    pub checker_cmd: Vec<String>,
    pub valuer_exe: FileRef,
}
