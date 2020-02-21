//! Implements Controller functionality related to getting tasks and publishing results
use super::{notify::Notifier, Controller, ExtendedInvokeRequest};
use crate::worker::{self, InvokeRequest};
use anyhow::Context;
use cfg::Limits;
use invoker_api::InvokeTask;
use std::{collections::HashMap, fs, path::PathBuf};
use uuid::Uuid;

fn merge_option<T: Copy>(place: &mut Option<T>, other: Option<T>) {
    if let Some(x) = other {
        place.replace(x);
    }
}

/// Merges several `Limits` object. Last element of slice will have maxinal proirity.
fn merge_limits(limits_set: &[Limits]) -> Limits {
    let mut res = Limits::default();
    for lim in limits_set {
        merge_option(&mut res.memory, lim.memory);
        merge_option(&mut res.process_count, lim.process_count);
        merge_option(&mut res.time, lim.time);
    }
    res
}

#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum InterpolateError {
    #[error("template syntax violation: {message}")]
    BadSyntax { message: &'static str },
    #[error("unknown key {key} in command template")]
    MissingKey { key: String },
}

/// Interpolates string by dictionary
///
/// Few examples of correct template strings:
/// - foo
/// - fo$(KeyName)
/// - fo$$$$(SomeKey)
///
/// Few examples of incorrect strings:
/// - $(
/// - $(SomeKey))
pub(crate) fn interpolate_string(
    string: &str,
    dict: &HashMap<String, String>,
) -> Result<String, InterpolateError> {
    let ak = aho_corasick::AhoCorasick::new_auto_configured(&["$(", ")"]);
    let matches = ak.find_iter(string);
    let mut out = String::new();
    let mut cur_pos = 0;
    let mut next_pat_id = 0;
    for m in matches {
        if m.pattern() != next_pat_id {
            return Err(InterpolateError::BadSyntax {
                message: "get pattern start while parsing pattern or pattern end outside of pattern",
            });
        }

        let chunk = &string[cur_pos..m.start()];
        cur_pos = m.end();
        if next_pat_id == 0 {
            out.push_str(chunk);
        } else {
            match dict.get(chunk) {
                Some(ref val) => {
                    out.push_str(val);
                }
                None => {
                    return Err(InterpolateError::MissingKey {
                        key: chunk.to_string(),
                    });
                }
            }
        }
        next_pat_id = 1 - next_pat_id;
    }
    let tail = &string[cur_pos..];
    out.push_str(tail);
    Ok(out)
}

fn interpolate_command(
    command: &cfg::Command,
    dict: &HashMap<String, String>,
) -> Result<worker::Command, InterpolateError> {
    let mut res: worker::Command = Default::default();
    for arg in &command.argv {
        let interp = interpolate_string(arg, dict)?;
        res.argv.push(interp);
    }
    for (name, val) in &command.env {
        let name = interpolate_string(name, dict)?;
        let val = interpolate_string(val, dict)?;
        res.env.push(format!("{}={}", name, val));
    }
    res.cwd = interpolate_string(&command.cwd, dict)?;
    Ok(res)
}

pub(crate) fn get_common_interpolation_dict(
    toolchain_cfg: &cfg::Toolchain,
) -> HashMap<String, String> {
    let mut dict = HashMap::new();
    dict.insert("Invoker.Id".to_string(), String::from("inv"));
    dict.insert(
        "Run.SourceFilePath".to_string(),
        PathBuf::from("/jjs")
            .join(&toolchain_cfg.filename)
            .display()
            .to_string(),
    );
    dict.insert("Run.BinaryFilePath".to_string(), "/jjs/build".into());
    dict
}

impl Controller {
    /// This functions queries all related data about run and returns InvokeRequest
    ///
    /// InvokeTask is not single source of trust, and some information needs to be taken from
    /// config.
    /// But ExtendedInvokeRequest **is** SSoT, and worker is completely isolated from other
    /// components.
    pub(super) fn fetch_run_info(
        &self,
        invoke_task: &InvokeTask,
        task_source_id: usize,
    ) -> anyhow::Result<ExtendedInvokeRequest> {
        let run_root = &invoke_task.run_dir;

        let mut run_metadata = HashMap::new();
        let judge_time = {
            let time = chrono::prelude::Utc::now();
            time.format("%Y-%m-%d %H:%M:%S").to_string()
        };
        run_metadata.insert("JudgeTimeUtc".to_string(), judge_time);
        {
            let mut buf = Uuid::encode_buffer();
            let s = invoke_task
                .invocation_id
                .to_hyphenated_ref()
                .encode_lower(&mut buf);
            run_metadata.insert("InvokeRequestId".to_string(), s.to_owned());
        }
        let prob_name = &invoke_task.problem_id;

        let problem_dir = self.config.sysroot.join("var/problems").join(&prob_name);

        let problem_manifest_path = problem_dir.join("manifest.json");

        let reader = std::io::BufReader::new(
            fs::File::open(problem_manifest_path).context("failed to read problem manifest")?,
        );

        let problem_data: pom::Problem =
            serde_json::from_reader(reader).context("invalid problem manifest")?;

        let toolchain_cfg = self
            .config
            .find_toolchain(&invoke_task.toolchain_id)
            .ok_or_else(|| anyhow::anyhow!("toolchain {} not found", &invoke_task.toolchain_id))?;

        let problem_cfg = self
            .config
            .find_problem(&invoke_task.problem_id)
            .ok_or_else(|| anyhow::anyhow!("problem {} not found", &invoke_task.problem_id))?;

        let compile_limits = merge_limits(&[toolchain_cfg.limits]);
        let execute_limits = merge_limits(&[problem_cfg.limits]);

        let run_source = run_root.join("source");
        let temp_invocation_dir = tempfile::tempdir().context("failed to create temporary dir")?;

        let out_dir = temp_invocation_dir.into_path();
        let interp_dict = {
            let mut dict = get_common_interpolation_dict(toolchain_cfg);
            for (k, v) in run_metadata {
                dict.insert(format!("Run.Meta.{}", k), v);
            }
            dict
        };
        let inv_req = InvokeRequest {
            compile_commands: toolchain_cfg
                .build_commands
                .iter()
                .map(|c| interpolate_command(c, &interp_dict))
                .collect::<Result<_, _>>()
                .context("invalid build commands template")?,
            execute_command: interpolate_command(&toolchain_cfg.run_command, &interp_dict)
                .context("invalid run command template")?,
            execute_limits,
            compile_limits,
            problem_dir,
            source_file_name: toolchain_cfg.filename.clone(),
            problem_data,
            run_source,
            out_dir,
            invocation_id: invoke_task.invocation_id,
            global_dir: self.config.install_dir.clone(),
            minion: self.minion.clone(),
            toolchains_dir: self.config.sysroot.join("opt"),
        };

        let req = ExtendedInvokeRequest {
            inner: inv_req,
            revision: invoke_task.revision,
            notifier: Notifier::new(invoke_task.status_update_callback.clone()),
            invocation_dir: invoke_task.invocation_dir.clone(),
            task_source_id,
        };
        Ok(req)
    }
}
