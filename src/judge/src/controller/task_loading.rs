//! Implements Controller functionality related to getting tasks and publishing results
use super::{
    notify::Notifier, Controller, JudgeRequestAndCallbacks, LoweredJudgeRequestExtensions,
};
use crate::{
    request_handler::{Command, LoweredJudgeRequest},
    worker,
};
use anyhow::Context;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use tracing::instrument;
use uuid::Uuid;

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
    command: &super::toolchains::Command,
    dict: &HashMap<String, String>,
    toolchain_spec: &super::toolchains::ToolchainSpec,
) -> Result<Command, InterpolateError> {
    let mut res: Command = Default::default();
    for arg in &command.argv {
        let interp = interpolate_string(arg, dict)?;
        res.argv.push(interp);
    }
    let mut used_env_vars = HashSet::new();
    for (name, val) in &command.env {
        let name = interpolate_string(name, dict)?;
        let val = interpolate_string(val, dict)?;
        res.env.push(format!("{}={}", name, val));
        used_env_vars.insert(name);
    }
    res.cwd = interpolate_string(&command.cwd, dict)?;
    for (default_key, default_val) in &toolchain_spec.env {
        if !used_env_vars.contains(default_key) {
            res.env.push(format!("{}={}", default_key, default_val));
        }
    }
    Ok(res)
}

pub(crate) fn get_common_interpolation_dict(
    toolchain: &super::toolchains::ToolchainSpec,
) -> HashMap<String, String> {
    let mut dict = HashMap::new();
    dict.insert("Invoker.Id".to_string(), String::from("inv"));
    dict.insert(
        "Run.SourceFilePath".to_string(),
        PathBuf::from("/jjs")
            .join(&toolchain.filename)
            .display()
            .to_string(),
    );
    dict.insert("Run.BinaryFilePath".to_string(), "/jjs/build".into());
    dict
}

impl Controller {
    /// This functions queries all related data about run and returns InvokeRequest
    ///
    /// `JudgeRequestAndCallbacks` is not single source of trust, and some
    /// information needs to be taken from config.
    /// But LoweredJudgeRequestWithExtensions **is** SSoT, and worker is
    /// completely isolated from other components.
    #[instrument(skip(self, judge_request_and_cbs))]
    pub(super) async fn lower_judge_request(
        &self,
        judge_request_and_cbs: &JudgeRequestAndCallbacks,
    ) -> anyhow::Result<(LoweredJudgeRequest, LoweredJudgeRequestExtensions)> {
        let mut run_metadata = HashMap::new();
        let judge_time = {
            let time = chrono::prelude::Utc::now();
            time.format("%Y-%m-%d %H:%M:%S").to_string()
        };
        run_metadata.insert("JudgeTimeUtc".to_string(), judge_time);
        {
            let mut buf = Uuid::encode_buffer();
            let s = judge_request_and_cbs
                .request
                .request_id
                .to_hyphenated_ref()
                .encode_lower(&mut buf);
            run_metadata.insert("InvokeRequestId".to_string(), s.to_owned());
        }
        let problem_name = &judge_request_and_cbs.request.problem_id;
        let (problem, problem_dir) = self
            .problem_loader
            .find(problem_name)
            .await
            .and_then(|opt| opt.ok_or_else(|| anyhow::anyhow!("unknown problem")))
            .with_context(|| format!("can not find problem `{}`", problem_name))?;

        let toolchain_info = self
            .toolchain_loader
            .resolve(&judge_request_and_cbs.request.toolchain_id)
            .await
            .context("toolchain loading error")?;

        let toolchain_spec = toolchain_info.get_spec();

        let temp_invocation_dir = tokio::task::spawn_blocking(tempfile::tempdir)
            .await
            .unwrap()
            .context("failed to create temporary dir")?;
        tracing::debug!(invocation_temporary_directory=%temp_invocation_dir.path().display());
        let run_source_temp_file = temp_invocation_dir.path().join("source");
        tokio::fs::write(
            &run_source_temp_file,
            &judge_request_and_cbs.request.run_source,
        )
        .await
        .context("unable to save run source in FS")?;

        let temp_invocation_dir = temp_invocation_dir.into_path();
        let interp_dict = {
            let mut dict = get_common_interpolation_dict(&toolchain_spec);
            for (k, v) in run_metadata {
                dict.insert(format!("Run.Meta.{}", k), v);
            }
            dict
        };
        tracing::debug!(interpolation_values=?interp_dict, "interpolation context");
        let compile_commands = toolchain_spec
            .build_commands
            .iter()
            .map(|c| interpolate_command(c, &interp_dict, &toolchain_spec))
            .collect::<Result<_, _>>()
            .context("invalid build commands template")?;
        let execute_command =
            interpolate_command(&toolchain_spec.run_command, &interp_dict, &toolchain_spec)
                .context("invalid run command template")?;
        let low_judge_request = LoweredJudgeRequest {
            compile_commands,
            execute_command,

            compile_limits: toolchain_spec.limits,
            problem_dir: problem_dir.to_path_buf(),
            source_file_name: toolchain_spec.filename.clone(),
            problem: problem.clone(),
            run_source: run_source_temp_file,
            out_dir: temp_invocation_dir.clone(),
            judge_request_id: judge_request_and_cbs.request.request_id,
            toolchain_dir: toolchain_info.path,
        };
        let exts = LoweredJudgeRequestExtensions {
            notifier: Notifier::new(
                judge_request_and_cbs.request.request_id,
                judge_request_and_cbs.callbacks.clone(),
            ),
            invocation_dir: temp_invocation_dir,
        };

        Ok((low_judge_request, exts))
    }
}
