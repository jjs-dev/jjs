use crate::{invoke_context::InvokeContext, os_util::make_anon_file};
use anyhow::Context;
use invoker_api::valuer_proto::{TestDoneNotification, ValuerResponse};
use slog_scope::warn;
use std::io::{BufRead, BufReader, BufWriter, Write};
pub(crate) struct Valuer<'a> {
    ctx: &'a dyn InvokeContext,
    child: std::process::Child,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
}

impl<'a> Valuer<'a> {
    pub(crate) fn new(ctx: &'a dyn InvokeContext) -> anyhow::Result<Valuer> {
        let valuer_exe = ctx.resolve_asset(&ctx.env().problem_data.valuer_exe);
        let mut cmd = std::process::Command::new(&valuer_exe);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());
        let public_comments = make_anon_file("PublicValuerComments");
        let private_comments = make_anon_file("PrivateValuerComments");
        cmd.env("JJS_VALUER_COMMENT_PUB", public_comments.to_string());
        cmd.env("JJS_VALUER_COMMENT_PRIV", private_comments.to_string());
        let work_dir = ctx.resolve_asset(&ctx.env().problem_data.valuer_cfg);
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

    pub(crate) fn write_problem_data(&mut self) -> anyhow::Result<()> {
        let problem_info = self.ctx.env().problem_data;
        writeln!(self.stdin, "{} ", problem_info.tests.len())?;
        self.stdin.flush()?;
        Ok(())
    }

    pub(crate) fn poll(&mut self) -> anyhow::Result<ValuerResponse> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).context("early eof")?;

        let response = serde_json::from_str(&line).context("failed to parse valuer message")?;

        Ok(response)
    }

    pub(crate) fn notify_test_done(
        &mut self,
        notification: TestDoneNotification,
    ) -> anyhow::Result<()> {
        writeln!(
            self.stdin,
            "{} {} {}",
            notification.test_id, notification.test_status.kind, notification.test_status.code
        )?;
        self.stdin.flush()?;
        Ok(())
    }
}

impl Drop for Valuer<'_> {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}
