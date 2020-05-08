use super::{Entity, Seal};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProblemBinding {
    /// Problem unique ID
    pub name: String,

    /// Problem ID in contest
    pub code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "kebab-case")]
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
    pub participants: Vec<String>,

    /// Whether contest is visible for users that are not included in
    /// contestants
    #[serde(rename = "vis-unreg")]
    pub unregistered_visible: bool,

    /// Whether contest is visible for anonymous users
    #[serde(rename = "vis-anon")]
    pub anon_visible: bool,

    /// Contest start time.
    /// If not set, it is `-inf`.
    #[serde(default)]
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Contest end time.
    /// If not time, it is `+inf`.
    #[serde(default)]
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Contest duration.
    /// For non-virtual contest, must be either
    /// omitted or be equal to `end_time` - `start_time`
    #[serde(default)]
    pub duration: Option<std::time::Duration>,

    /// If enabled, contest is virtual
    /// Virtual contest is started by user.
    /// User will not be able to interact with contest until they `takePart` in
    /// it. For virtual contest, `start_time` and `end_time` define a period
    /// of time when user can start their participation.
    #[serde(rename = "virtual")]
    #[serde(default)]
    pub is_virtual: bool,
}

impl Seal for Contest {}
impl Entity for Contest {
    fn name(&self) -> &str {
        &self.id
    }
}
