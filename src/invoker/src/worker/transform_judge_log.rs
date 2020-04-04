use crate::worker::{InvokeRequest, Worker};
use anyhow::Context;
use invoker_api::{
    judge_log, status_codes, valuer_proto::TestVisibleComponents, Status, StatusKind,
};
use std::io::Read;

impl Worker {
    /// Go from valuer judge log to invoker judge log
    // Bug in clippy: https://github.com/rust-lang/rust-clippy/issues/5368
    #[allow(clippy::verbose_file_reads)]
    pub(super) fn process_judge_log(
        &self,
        valuer_log: &invoker_api::valuer_proto::JudgeLog,
        req: &InvokeRequest,
        test_results: &[(pom::TestId, crate::worker::exec_test::ExecOutcome)],
    ) -> anyhow::Result<judge_log::JudgeLog> {
        let resource_usage_by_test = {
            let mut map = std::collections::HashMap::new();
            for (k, v) in test_results {
                map.insert(*k, v.resource_usage);
            }
            map
        };
        let mut persistent_judge_log = judge_log::JudgeLog::default();
        let status = if valuer_log.is_full {
            Status {
                kind: StatusKind::Accepted,
                code: status_codes::ACCEPTED.to_string(),
            }
        } else {
            Status {
                kind: StatusKind::Rejected,
                code: status_codes::PARTIAL_SOLUTION.to_string(),
            }
        };
        persistent_judge_log.status = status;
        persistent_judge_log.kind = valuer_log.kind;
        persistent_judge_log.score = valuer_log.score;
        // now fill compile_stdout and compile_stderr in judge_log
        {
            let mut compile_stdout = Vec::new();
            let mut compile_stderr = Vec::new();
            let compile_dir = req.out_dir.join("compile");
            for i in 0.. {
                let stdout_file = compile_dir.join(format!("stdout-{}.txt", i));
                let stderr_file = compile_dir.join(format!("stderr-{}.txt", i));
                if !stdout_file.exists() || !stderr_file.exists() {
                    break;
                }
                let mut stdout_file =
                    std::fs::File::open(stdout_file).context("failed to open output log")?;
                let mut stderr_file =
                    std::fs::File::open(stderr_file).context("failed to open errors log")?;
                stdout_file
                    .read_to_end(&mut compile_stdout)
                    .context("failed to read output log")?;
                stderr_file
                    .read_to_end(&mut compile_stderr)
                    .context("failed to read errors log")?;
            }
            persistent_judge_log.compile_stdout = base64::encode(&compile_stdout);
            persistent_judge_log.compile_stderr = base64::encode(&compile_stderr);
        }
        // for each test, if valuer allowed, add stdin/stdout/stderr etc to judge_log
        {
            for item in &valuer_log.tests {
                let mut new_item = judge_log::JudgeLogTestRow {
                    test_id: item.test_id,
                    test_answer: None,
                    test_stdout: None,
                    test_stderr: None,
                    test_stdin: None,
                    status: None,
                    time_usage: None,
                    memory_usage: None,
                };
                let test_local_dir = req.step_dir(Some(item.test_id.get()));
                if item.components.contains(TestVisibleComponents::TEST_DATA) {
                    let test_file = &req.problem.tests[item.test_id].path;
                    let test_file = req.resolve_asset(&test_file);
                    let test_data = std::fs::read(test_file).context("failed to read test data")?;
                    let test_data = base64::encode(&test_data);
                    new_item.test_stdin = Some(test_data);
                }
                if item.components.contains(TestVisibleComponents::OUTPUT) {
                    let stdout_file = test_local_dir.join("stdout.txt");
                    let stderr_file = test_local_dir.join("stderr.txt");
                    //println!("DEBUG: stdout_file={}", stdout_file.display());
                    let sol_stdout =
                        std::fs::read(stdout_file).context("failed to read solution stdout")?;
                    let sol_stderr =
                        std::fs::read(stderr_file).context("failed to read solution stderr")?;
                    let sol_stdout = base64::encode(&sol_stdout);
                    let sol_stderr = base64::encode(&sol_stderr);
                    new_item.test_stdout = Some(sol_stdout);
                    new_item.test_stderr = Some(sol_stderr);
                }
                if item.components.contains(TestVisibleComponents::ANSWER) {
                    let answer_ref = &req.problem.tests[item.test_id].correct;
                    if let Some(answer_ref) = answer_ref {
                        let answer_file = req.resolve_asset(answer_ref);
                        let answer =
                            std::fs::read(answer_file).context("failed to read correct answer")?;
                        let answer = base64::encode(&answer);
                        new_item.test_answer = Some(answer);
                    }
                }
                if item.components.contains(TestVisibleComponents::STATUS) {
                    new_item.status = Some(item.status.clone());
                }
                if let Some(resource_usage) = resource_usage_by_test.get(&item.test_id) {
                    if item
                        .components
                        .contains(TestVisibleComponents::RESOURCE_USAGE)
                    {
                        new_item.memory_usage = resource_usage.memory;
                        new_item.time_usage = resource_usage.time;
                    }
                }
                persistent_judge_log.tests.push(new_item);
            }
            persistent_judge_log
                .tests
                .sort_by(|a, b| a.test_id.cmp(&b.test_id));
        }
        {
            for item in &valuer_log.subtasks {
                persistent_judge_log
                    .subtasks
                    .push(judge_log::JudgeLogSubtaskRow {
                        subtask_id: item.subtask_id,
                        score: Some(item.score),
                    });
            }
            persistent_judge_log
                .subtasks
                .sort_by(|a, b| a.subtask_id.0.cmp(&b.subtask_id.0));
        }
        // note that we do not filter subtasks connected staff,
        // because such filtering is done by Valuer.

        Ok(persistent_judge_log)
    }
}
