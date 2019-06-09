use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Test {
    pub path: String,
    pub correct: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub name: String,
    pub tests: Vec<Test>,
    pub checker: String,
}
