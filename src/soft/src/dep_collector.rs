use anyhow::Context;
use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct DepCollector {
    files: HashSet<String>,
}

fn decode_path(p: &serde_json::Value) -> anyhow::Result<Option<String>> {
    if p.get("kind").and_then(|val| val.as_str()) == Some("unknown") {
        return Ok(None);
    }
    p.get("data")
        .and_then(|data| data.as_str())
        .map(ToString::to_string)
        .context("bad ktrace path output")
        .map(Some)
}

impl DepCollector {
    pub(crate) fn process_log_item(&mut self, s: &serde_json::Value) -> anyhow::Result<usize> {
        let cnt_before = self.files.len();
        static MSG: &str = "unexpected ktrace output";
        let value = s.as_object().context(MSG)?.get("payload").context(MSG)?;

        let kind = value.get("kind").context(MSG)?;
        match kind.as_str().context(MSG)? {
            "attach" | "exit" | "signal" => return Ok(0),
            "sysenter" | "sysexit" => (),
            other => anyhow::bail!("unknown event kind: {}", other),
        }
        let value = value.get("data").context(MSG)?;
        let syscall_name = value
            .pointer("/decoded/name")
            .context("decoded syscall info missing")?
            .as_str()
            .context(MSG)?;
        let syscall_args = value
            .pointer("/decoded/args")
            .context(MSG)?
            .as_array()
            .context(MSG)?;
        let syscall_is_error = value
            .pointer("/decoded/ret/kind")
            .map(|val| val.as_str() == Some("error"));
        if syscall_is_error.unwrap_or(false) {
            return Ok(0);
        }

        let mut process_path = |path_val| -> anyhow::Result<()> {
            if let Some(path) = decode_path(path_val)? {
                self.files.insert(path);
            }
            Ok(())
        };

        match syscall_name {
            "execve" => {
                // read argv[0]
                process_path(&syscall_args[0])?;
            }
            "access" => {
                // accessed file
                process_path(&syscall_args[0])?;
            }
            "openat" => {
                // opened file
                process_path(&syscall_args[1])?;
            }
            "open" => {
                // opened file
                process_path(&syscall_args[0])?;
            }
            "lstat" => {
                // file stats were requested for
                process_path(&syscall_args[0])?;
            }
            _ => {}
        }

        let cnt_after = self.files.len();

        Ok(cnt_after - cnt_before)
    }

    pub(crate) fn count(&self) -> usize {
        self.files.len()
    }
}

impl IntoIterator for DepCollector {
    type IntoIter = <HashSet<String> as IntoIterator>::IntoIter;
    type Item = String;

    fn into_iter(self) -> Self::IntoIter {
        self.files.into_iter()
    }
}
