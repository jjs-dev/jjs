use crate::worker::InvokeRequest;
use anyhow::{bail, Context};
use invoker_api::valuer_proto::{ProblemInfo, TestDoneNotification, ValuerResponse};
use slog_scope::warn;
use std::os::unix::io::IntoRawFd;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
pub(crate) struct Valuer {
    stdin: BufWriter<tokio::process::ChildStdin>,
    stdout: BufReader<tokio::process::ChildStdout>,
    // ties lifetime of valuer instance to `Valuer` lifetime
    _child: tokio::process::Child,
}

impl Valuer {
    pub(crate) fn new(req: &InvokeRequest) -> anyhow::Result<Valuer> {
        let valuer_exe = req.resolve_asset(&req.problem.valuer_exe);
        let mut cmd = tokio::process::Command::new(&valuer_exe);
        cmd.kill_on_drop(true);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());
        cmd.env("JJS_VALUER", "1");
        cmd.env("RUST_LOG", "info,svaluer=debug");
        let work_dir = req.resolve_asset(&req.problem.valuer_cfg);
        if work_dir.exists() {
            cmd.current_dir(&work_dir);
        } else {
            warn!(
                "Not setting current dir for valuer because path specified ({}) does not exists",
                work_dir.display()
            );
        }
        let log = std::fs::File::create(req.out_dir.join("valuer-log.txt"))
            .context("failed to create valuer log file")?
            .into_raw_fd();
        unsafe {
            cmd.pre_exec(move || {
                nix::unistd::dup3(log, libc::STDERR_FILENO, nix::fcntl::OFlag::empty())
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
                Ok(())
            });
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
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
            _child: child,
        };

        Ok(val)
    }

    async fn write_val(&mut self, msg: impl serde::Serialize) -> anyhow::Result<()> {
        let mut msg = serde_json::to_string(&msg).context("failed to serialize")?;
        if msg.contains('\n') {
            bail!("bug: serialized message is not oneline");
        }
        msg.push('\n');
        self.stdin
            .write_all(msg.as_bytes())
            .await
            .context("failed to write message")?;
        self.stdin
            .flush()
            .await
            .context("failed to flush valuer stdin")?;
        Ok(())
    }

    pub(crate) async fn write_problem_data(&mut self, req: &InvokeRequest) -> anyhow::Result<()> {
        let proto_problem_info = ProblemInfo {
            tests: req
                .problem
                .tests
                .iter()
                .map(|test_spec| test_spec.group.clone())
                .collect(),
        };
        self.write_val(proto_problem_info).await
    }

    pub(crate) async fn poll(&mut self) -> anyhow::Result<ValuerResponse> {
        let mut line = String::new();
        let read_line_fut = self.stdout.read_line(&mut line);
        match tokio::time::timeout(std::time::Duration::from_secs(15), read_line_fut).await {
            Ok(read) => {
                read.context("early eof")?;
            }
            Err(_elapsed) => {
                bail!("valuer response timed out");
            }
        }
        let response = serde_json::from_str(&line).context("failed to parse valuer message")?;

        Ok(response)
    }

    pub(crate) async fn notify_test_done(
        &mut self,
        notification: TestDoneNotification,
    ) -> anyhow::Result<()> {
        self.write_val(notification).await
    }
}
