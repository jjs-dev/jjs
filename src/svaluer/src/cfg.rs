use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeedbackKind {
    /// no feedback provided
    Hidden,
    /// Only summary is provided
    Brief,
    /// Full feedback is provided
    Full,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum GroupRef {
    ByName(String),
    ById(u32),
}

fn default_run_to_first_failure() -> bool {
    true
}

#[derive(Deserialize, Serialize)]
pub struct Group {
    /// Group name.
    /// It is user to refer from other groups.
    pub name: String,
    /// Determines what information will be provided to contestant
    pub feedback: FeedbackKind,
    /// Tag to find tests in this group. If none, same as `name`
    pub tests_tag: Option<String>,
    /// Stop running group if some test failed
    #[serde(default = "default_run_to_first_failure")]
    pub run_to_first_failure: bool,
    /// Group score
    pub score: u32,
    /// Required groups
    #[serde(default)]
    pub deps: Vec<GroupRef>,
}

impl Group {
    pub fn tests_tag(&self) -> &str {
        self.tests_tag.as_deref().unwrap_or(&self.name)
    }
}

/// SValuer config
/// # Offline tests
/// For offline tests, contestant is not provided with feedback.
/// To activate, set `open_tests_count` and `open_tests_score`.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub groups: Vec<Group>,
}

const MSG_INVALID_GROUP_REF: &str = "GroupRef refers to nonexistent group";
const MSG_CIRCULAR_REF: &str = "group dependencies have cycle";

fn dfs(graph: &[Vec<usize>], used: &mut [u8], has_cycle: &mut bool, v: usize) {
    used[v] = 1;
    for &w in &graph[v] {
        if used[w] == 0 {
            dfs(graph, used, has_cycle, w);
        } else if used[w] == 1 {
            *has_cycle = true;
        }
    }
    used[v] = 2;
}

impl Config {
    pub fn get_group(&self, dep: &GroupRef) -> Option<usize> {
        match dep {
            GroupRef::ById(id) => {
                if (*id as usize) < self.groups.len() {
                    Some(*id as usize)
                } else {
                    None
                }
            }
            GroupRef::ByName(name) => self.groups.iter().position(|g| &g.name == name),
        }
    }

    pub fn validate(&self, error_sink: &mut Vec<String>) {
        let mut group_dep_graph: Vec<Vec<usize>> = Vec::new();

        group_dep_graph.resize_with(self.groups.len(), Vec::new);

        for (i, g) in self.groups.iter().enumerate() {
            for dep in &g.deps {
                match self.get_group(dep) {
                    Some(j) => {
                        group_dep_graph[i].push(j);
                    }
                    None => {
                        error_sink.push(MSG_INVALID_GROUP_REF.to_string());
                    }
                }
            }
        }
        let mut has_cycle = false;
        let mut used = vec![0; self.groups.len()];
        for i in 0..self.groups.len() {
            if used[i] == 0 {
                dfs(&group_dep_graph, &mut used, &mut has_cycle, 0);
            }
        }
        if has_cycle {
            error_sink.push(MSG_CIRCULAR_REF.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod validate {
        use super::*;

        fn check_errs(config: &str, errs: &[&str]) {
            let cfg: Config = serde_yaml::from_str(config).unwrap();
            let mut sink = Vec::new();

            cfg.validate(&mut sink);
            if sink != errs {
                panic!("expected {:?}, got {:?}", errs, sink);
            }
        }
        #[test]
        fn test_ok() {
            check_errs(
                "
groups:
 - name: grp
   feedback: full
   score: 0
            ",
                &[],
            );
        }

        #[test]
        fn test_invalid_numeric_ref() {
            check_errs(
                "
groups:
 - name: g1
   feedback: full            
   score: 15
 - name: g2
   feedback: full
   score: 85
   deps:
     - 2
   ",
                &[MSG_INVALID_GROUP_REF],
            );
        }

        #[test]
        fn test_invalid_named_ref() {
            check_errs(
                "
groups:
  - name: foo
    feedback: full
    score: 15       
    deps:
      - bar     
            ",
                &[MSG_INVALID_GROUP_REF],
            );
        }

        #[test]
        fn test_circular_ref() {
            check_errs(
                "
groups:
  - name: foo
    feedback: full
    score: 50
    deps:
      - bar
  - name: bar
    feedback: hidden
    score: 50
    deps:
      - 0            
            ",
                &[MSG_CIRCULAR_REF],
            )
        }
    }
}
