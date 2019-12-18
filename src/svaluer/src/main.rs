//! Simple valuer
use pom::TestId;
use std::collections::HashSet;

/// CLI-based driver, useful for manual testing valuer config
struct TermDriver {
    current_tests: HashSet<TestId>,
}

mod term_driver {
    use super::TermDriver;
    use anyhow::{Context, Result};
    use invoker_api::valuer_proto;
    use pom::TestId;
    use std::{
        io::{stdin, stdout, Write},
        str::FromStr,
    };
    fn read_value<T: FromStr>(what: impl AsRef<str>) -> Result<T>
    where
        <T as FromStr>::Err: std::fmt::Display,
    {
        let mut user_input = String::new();
        loop {
            print!("{}> ", what.as_ref());
            stdout().flush()?;
            user_input.clear();
            stdin()
                .read_line(&mut user_input)
                .context("failed to read line")?;
            let user_input = user_input.trim();
            match user_input.parse() {
                // These are different Ok's: One is anyhow::Result::Ok, other is Result<.., <T as FromStr>::Err>>
                Ok(x) => break Ok(x),
                Err(err) => {
                    eprintln!("failed to parse your input: {}. Please, enter again.", err);
                    continue;
                }
            }
        }
    }
    impl svaluer::ValuerDriver for TermDriver {
        fn problem_info(&mut self) -> Result<valuer_proto::ProblemInfo> {
            let test_count = read_value("test count")?;
            let info = valuer_proto::ProblemInfo { test_count };
            Ok(info)
        }

        fn send_command(&mut self, resp: &valuer_proto::ValuerResponse) -> Result<()> {
            match resp {
                valuer_proto::ValuerResponse::Finish {
                    score,
                    judge_log,
                    treat_as_full,
                } => {
                    println!("Judging finished");
                    println!("Score: {}", *score);
                    if *treat_as_full {
                        println!("Full solution");
                    } else {
                        println!("Partial solution");
                    }
                    // TODO print judge log too
                    let _ = judge_log;
                }
                valuer_proto::ValuerResponse::LiveScore { score } => {
                    println!("Current score: {}", *score);
                }
                valuer_proto::ValuerResponse::Test { test_id, live } => {
                    println!("Run should be executed on test {}", test_id.get());
                    if *live {
                        println!("Current test: {}", test_id.get());
                    }
                    let not_dup = self.current_tests.insert(*test_id);
                    assert!(not_dup);
                }
            }
            Ok(())
        }

        fn poll_notification(&mut self) -> Result<Option<valuer_proto::TestDoneNotification>> {
            fn create_status(ok: bool) -> invoker_api::Status {
                if ok {
                    invoker_api::Status {
                        code: "OK".to_string(),
                        kind: invoker_api::StatusKind::Accepted,
                    }
                } else {
                    invoker_api::Status {
                        code: "NOT_OK".to_string(),
                        kind: invoker_api::StatusKind::Rejected,
                    }
                }
            }

            fn read_status(tid: TestId) -> Result<valuer_proto::TestDoneNotification> {
                let outcome = read_value(format!("test {} status", tid.get()))?;
                let test_status = create_status(outcome);
                Ok(valuer_proto::TestDoneNotification {
                    test_id: tid,
                    test_status,
                })
            }
            match self.current_tests.len() {
                0 => Ok(None),
                1 => {
                    let tid = self.current_tests.drain().next().unwrap();
                    Ok(Some(read_status(tid)?))
                }
                _ => {
                    let test_id = loop {
                        let tid: std::num::NonZeroU32 = read_value("next finished test")?;
                        if !self.current_tests.remove(&TestId(tid)) {
                            eprintln!(
                                "Test {} was already finished or is not requested to run",
                                tid.get()
                            );
                            eprintln!("Current tests: {:?}", &self.current_tests);
                            continue;
                        }
                        break TestId(tid);
                    };
                    Ok(Some(read_status(test_id)?))
                }
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut driver = TermDriver {
        current_tests: HashSet::new(),
    };
    let valuer = svaluer::SimpleValuer::new(&mut driver)?;
    valuer.exec()?;
    Ok(())
}
