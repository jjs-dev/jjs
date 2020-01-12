use serde::{Deserialize, Serialize};

/// SValuer config
/// # Offline tests
/// For offline tests, contestant is not provided with feedback.
/// To activate, set `open_tests_count` and `open_tests_score`.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// First `samples_count` tests will be considered samples.
    /// For samples, full feedback is provided.
    #[serde(default = "default_samples_count")]
    pub samples_count: u32,

    /// Only for first `open_tests_count` tests contestant will have feedback.
    pub open_tests_count: Option<u32>,
    /// How many points run gains if it passed all open tests
    pub open_tests_score: Option<u32>,
}

const MSG_OFFLINE_PARTIAL_CONFIG: &str =
    "offline mode is requested, but not all required fields are provided";

impl Config {
    pub fn validate(&self, error_sink: &mut Vec<String>) {
        let offline_tests_enabled =
            self.open_tests_count.is_some() || self.open_tests_score.is_some();
        let full_open_tests_config_given =
            self.open_tests_count.is_none() || self.open_tests_score.is_none();
        if offline_tests_enabled && full_open_tests_config_given {
            error_sink.push(MSG_OFFLINE_PARTIAL_CONFIG.to_string());
        }
    }
}

fn default_samples_count() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    mod validate {
        use super::*;
        #[test]
        fn test_ok() {
            let cfg = Config {
                open_tests_count: Some(10),
                open_tests_score: Some(50),
                samples_count: 2,
            };
            let mut sink = Vec::new();
            cfg.validate(&mut sink);
            assert!(sink.is_empty());
        }

        #[test]
        fn test_incorrect_open_tests_mode() {
            let cfg = Config {
                samples_count: 2,
                open_tests_count: Some(10),
                open_tests_score: None,
            };
            let mut sink = Vec::new();
            cfg.validate(&mut sink);
            assert_eq!(sink, [MSG_OFFLINE_PARTIAL_CONFIG]);
        }
    }
}
