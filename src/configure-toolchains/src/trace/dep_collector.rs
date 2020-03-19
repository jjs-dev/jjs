use anyhow::Context;
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct DepCollector {
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
    pub(super) fn process_log_item(&mut self, s: &serde_json::Value) -> anyhow::Result<usize> {
        let cnt_before = self.files.len();

        let value = s
            .as_object()
            .context("event is not object")?
            .get("payload")
            .context("payload missing")?;

        let kind = value.get("kind").context("kind missing")?;
        match kind.as_str().context("kind is not string")? {
            "attach" | "exit" | "signal" => return Ok(0),
            "sysenter" | "sysexit" => (),
            other => anyhow::bail!("unknown event kind: {}", other),
        }
        let value = value.get("data").context("data missing")?;
        let syscall_name = value
            .pointer("/decoded/name")
            .context("decoded syscall info missing")?
            .as_str()
            .context("syscall name is not string")?;
        let syscall_args = value
            .pointer("/decoded/args")
            .context("decoded args missing")?
            .as_array()
            .context("decoded args is not array")?;
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
}

impl IntoIterator for DepCollector {
    type IntoIter = <HashSet<String> as IntoIterator>::IntoIter;
    type Item = String;

    fn into_iter(self) -> Self::IntoIter {
        self.files.into_iter()
    }
}
