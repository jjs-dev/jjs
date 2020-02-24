use crate::worker::InvokeRequest;
use anyhow::{bail, Context};
use invoker_api::valuer_proto::{ProblemInfo, TestDoneNotification, ValuerResponse};
use slog_scope::warn;
use std::{
    convert::TryInto,
    io::{BufRead, BufReader, BufWriter, Write},
    os::unix::{io::IntoRawFd, process::CommandExt},
};
pub(crate) struct Valuer {
    child: std::process::Child,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
}

impl Valuer {
    pub(crate) fn new(req: &InvokeRequest) -> anyhow::Result<Valuer> {
        let valuer_exe = req.resolve_asset(&req.problem.valuer_exe);
        let mut cmd = std::process::Command::new(&valuer_exe);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());
        cmd.env("JJS_VALUER", "1");
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

    pub(crate) fn write_problem_data(&mut self, req: &InvokeRequest) -> anyhow::Result<()> {
        let proto_problem_info = ProblemInfo {
            test_count: req
                .problem
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

impl Drop for Valuer {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}
