use crate::{
    err::ErrorBox,
    inter_api::{ValuerNotification, ValuerResponse},
    invoke_context::InvokeContext,
};
use snafu::ResultExt;
use snafu_derive::Snafu;

use crate::os_util::make_anon_file;
use std::io::{BufRead, BufReader, BufWriter, Write};

pub(crate) struct Valuer<'a> {
    ctx: InvokeContext<'a>,
    child: std::process::Child,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
}

#[derive(Debug, Snafu)]
enum ParseError {
    WrongArgCount { got: usize, expected: usize },
    UnknownMessage { head: String, tail: Vec<String> },
    NumParseFail { source: std::num::ParseIntError },
    Range { lhs: i64, rhs: i64, got: i64 },
    Other { message: String },
}

impl<'a> Valuer<'a> {
    pub(crate) fn new(ctx: InvokeContext<'a>) -> Result<Valuer, ErrorBox> {
        let valuer_exe = ctx.get_asset_path(&ctx.problem_data.valuer_exe);
        let mut cmd = std::process::Command::new(valuer_exe);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());
        let public_comments = make_anon_file("PublicValuerComments");
        let private_comments = make_anon_file("PrivateValuerComments");
        cmd.env("JJS_VALUER_COMMENT_PUB", public_comments.to_string());
        cmd.env("JJS_VALUER_COMMENT_PRIV", private_comments.to_string());
        let mut child = cmd.spawn().map_err(Box::new)?;
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

    fn write_problem_data(&mut self) -> Result<(), ErrorBox> {
        let problem_info = &self.ctx.problem_data;
        writeln!(self.stdin, "{} ", problem_info.tests.len())?;
        self.stdin.flush()?;
        Ok(())
    }

    fn read_response(&mut self) -> Result<ValuerResponse, ErrorBox> {
        let mut line = String::new();
        self.stdout.read_line(&mut line)?;

        let items: Vec<_> = line.split_whitespace().collect();
        let res = match items[0] {
            "RUN" => {
                if items.len() != 2 {
                    return Err(Box::new(ParseError::WrongArgCount {
                        expected: 1,
                        got: items.len() - 1,
                    }));
                }
                let test_id: u32 = items[1].parse().context(NumParseFail)?;
                ValuerResponse::Test { test_id }
            }
            "DONE" => {
                if items.len() != 4 {
                    return Err(Box::new(ParseError::WrongArgCount {
                        expected: 2,
                        got: items.len() - 1,
                    }));
                }
                let score: u16 = items[1].parse().context(NumParseFail)?;
                let is_full: i8 = items[2].parse().context(NumParseFail)?;
                let num_judge_log_rows: usize = items[3].parse().context(NumParseFail)?;

                if score > 100 {
                    return Err(Box::new(ParseError::Range {
                        lhs: 0,
                        rhs: 100,
                        got: score.into(),
                    }));
                }
                if is_full < 0 || is_full > 1 {
                    return Err(Box::new(ParseError::Range {
                        lhs: 0,
                        rhs: 1,
                        got: is_full.into(),
                    }));
                }

                let mut tests = Vec::new();
                for _ in 0..num_judge_log_rows {
                    line.clear();
                    self.stdout.read_line(&mut line)?;

                    tests.push(line.parse()?);
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
                return Err(Box::new(ParseError::UnknownMessage {
                    head: items[0].to_string(),
                    tail: items[1..].iter().map(|x| x.to_string()).collect(),
                }));
            }
        };
        Ok(res)
    }

    pub(crate) fn initial_test(&mut self) -> Result<ValuerResponse, ErrorBox> {
        self.write_problem_data()?;
        self.read_response()
    }

    pub(crate) fn notify_test_done(
        &mut self,
        notification: ValuerNotification,
    ) -> Result<ValuerResponse, ErrorBox> {
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
