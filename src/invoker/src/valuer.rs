use crate::{
    inter_api::{ValuerNotification, ValuerResponse},
    invoke_context::InvokeContext,
    os_util::make_anon_file,
};
use anyhow::{bail, Context};
use slog_scope::warn;
use std::io::{BufRead, BufReader, BufWriter, Write};

pub(crate) struct Valuer<'a> {
    ctx: InvokeContext<'a>,
    child: std::process::Child,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
}

impl<'a> Valuer<'a> {
    pub(crate) fn new(ctx: InvokeContext<'a>) -> anyhow::Result<Valuer> {
        let valuer_exe = ctx.get_asset_path(&ctx.problem_data.valuer_exe);
        let mut cmd = std::process::Command::new(&valuer_exe);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());
        let public_comments = make_anon_file("PublicValuerComments");
        let private_comments = make_anon_file("PrivateValuerComments");
        cmd.env("JJS_VALUER_COMMENT_PUB", public_comments.to_string());
        cmd.env("JJS_VALUER_COMMENT_PRIV", private_comments.to_string());
        let work_dir = ctx.get_asset_path(&ctx.problem_data.valuer_cfg);
        if work_dir.exists() {
            cmd.current_dir(&work_dir);
        } else {
            warn!(
                "Not setting current dir for valuer because path specified ({}) does not exists",
                work_dir.display()
            );
        }
        let mut child = cmd.spawn().with_context(|| {
            format!(
                "failed to spawn valuer {} (requested current dir {})",
                valuer_exe.display(),
                work_dir.display()
            )
        })?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let val = Valuer {
            ctx,
            child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
        };

        Ok(val)
    }

    fn write_problem_data(&mut self) -> anyhow::Result<()> {
        let problem_info = &self.ctx.problem_data;
        writeln!(self.stdin, "{} ", problem_info.tests.len())?;
        self.stdin.flush()?;
        Ok(())
    }

    fn read_response(&mut self) -> anyhow::Result<ValuerResponse> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).context("early eof")?;

        let items: Vec<_> = line.split_whitespace().collect();
        let res = match items[0] {
            "RUN" => {
                if items.len() != 2 {
                    bail!("RUN: expected 1 item, got {}", items.len() - 1);
                }
                let test_id: u32 = items[1].parse().context("RUN: test_id is not u32")?;
                ValuerResponse::Test { test_id }
            }
            "DONE" => {
                if items.len() != 4 {
                    bail!("DONE: expected 4 items, got {}", items.len() - 1);
                }
                let score: u16 = items[1].parse().context("DONE: score is not u16")?;
                let is_full: i8 = items[2].parse().context("DONE: is_full is not flag")?;
                let num_judge_log_rows: usize = items[3]
                    .parse()
                    .context("Done: num_judge_log_rows is not uint")?;

                if score > 100 {
                    bail!("score is bigger than 100");
                }
                if is_full < 0 || is_full > 1 {
                    bail!("DONE: is_full must be 0 or 1");
                }

                let mut tests = Vec::new();
                for _ in 0..num_judge_log_rows {
                    line.clear();
                    self.stdout
                        .read_line(&mut line)
                        .context("failed to read judge log row")?;
                    tests.push(line.parse().context("failed to parse judge log row")?);
                }
                ValuerResponse::Finish {
                    score: score.into(),
                    treat_as_full: is_full == 1,
                    judge_log: crate::judge_log::JudgeLog {
                        tests,
                        compile_stdout: String::new(),
                        name: "main".to_string(),
                        compile_stderr: String::new(),
                    },
                }
            }
            _ => {
                bail!("unknown message {}", items[0]);
            }
        };
        Ok(res)
    }

    pub(crate) fn initial_test(&mut self) -> anyhow::Result<ValuerResponse> {
        self.write_problem_data()?;
        self.read_response()
    }

    pub(crate) fn notify_test_done(
        &mut self,
        notification: ValuerNotification,
    ) -> anyhow::Result<ValuerResponse> {
        writeln!(
            self.stdin,
            "{} {} {}",
            notification.test_id, notification.test_status.kind, notification.test_status.code
        )?;
        self.stdin.flush()?;
        self.read_response()
    }
}

impl Drop for Valuer<'_> {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}
