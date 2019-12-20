use crate::{invoke_context::InvokeContext, os_util::make_anon_file};
use anyhow::{bail, Context};
use invoker_api::valuer_proto::{ProblemInfo, TestDoneNotification, ValuerResponse};
use slog_scope::warn;
use std::{
    convert::TryInto,
    io::{BufRead, BufReader, BufWriter, Write},
};
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
        cmd.env("JJS_VALUER", "1");
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

    fn write_val(&mut self, msg: impl serde::Serialize) -> anyhow::Result<()> {
        let mut msg = serde_json::to_string(&msg).context("failed to serialize")?;
        if msg.contains('\n') {
            bail!("bug: serialized message is not oneline");
        }
        msg.push('\n');
        self.stdin
            .write_all(msg.as_bytes())
            .context("failed to write message")?;
        self.stdin.flush().context("failed to flush valuer stdin")?;
        Ok(())
    }

    pub(crate) fn write_problem_data(&mut self) -> anyhow::Result<()> {
        let problem_info = self.ctx.env().problem_data;
        let proto_problem_info = ProblemInfo {
            test_count: problem_info
                .tests
                .len()
                .try_into()
                .expect("wow such many tests"),
        };
        self.write_val(proto_problem_info)
    }

    pub(crate) fn poll(&mut self) -> anyhow::Result<ValuerResponse> {
        let mut line = String::new();
        // TODO: timeout
        self.stdout.read_line(&mut line).context("early eof")?;

        let response = serde_json::from_str(&line).context("failed to parse valuer message")?;

        Ok(response)
    }

    pub(crate) fn notify_test_done(
        &mut self,
        notification: TestDoneNotification,
    ) -> anyhow::Result<()> {
        self.write_val(notification)
    }
}

impl Drop for Valuer<'_> {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}
