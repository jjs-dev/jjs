use super::{Entity, Seal};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProblemBinding {
    /// Problem unique ID
    pub name: String,

    /// Problem ID in contest
    pub code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Contest {
    pub title: String,

    pub id: String,

    /// Information about problems, not related to judging
    /// process (which is controlled by problem itself)
    pub problems: Vec<ProblemBinding>,

    /// List of groups of judges
    /// Judges will have full capabilities in this contest
    pub judges: Vec<String>,

    /// Which group members are considered registered for contest
    pub group: Vec<String>,

    /// Whether contest is visible for users that are not included in contestants
    #[serde(rename = "vis-unreg")]
    pub unregistered_visible: bool,

    /// Whether contest is visible for anonymous users
    #[serde(rename = "vis-anon")]
    pub anon_visible: bool,
}

impl Seal for Contest {}
impl Entity for Contest {
    fn name(&self) -> &str {
        &self.id
    }
}
