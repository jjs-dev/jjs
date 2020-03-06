use frontend_api::Client;
use graphql_client::GraphQLQuery;
use serde_json::Value;
use std::process::exit;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opt {
    /// problem code, e.g. "A"
    #[structopt(long, short = "p")]
    problem: String,
    #[structopt(long, short = "t")]
    toolchain: String,
    #[structopt(long, short = "f")]
    filename: String,
    #[structopt(long, short = "c")]
    contest: String,
    #[structopt(long, short = "n", default_value = "1")]
    count: u32,
}

fn resolve_toolchain(client: &Client, name: &str) -> String {
    let vars = crate::queries::list_toolchains::Variables {};

    let res = client
        .query::<_, crate::queries::list_toolchains::ResponseData>(
            &crate::queries::ListToolchains::build_query(vars),
        )
        .expect("network error")
        .into_result();
    let res = res.expect("Couldn't get toolchain information");
    for tc in res.toolchains {
        if tc.id == name {
            return tc.id;
        }
    }
    panic!("Couldn't find toolchain {}", name);
}

fn resolve_problem(client: &Client, contest_name: &str, problem_code: &str) -> (String, String) {
    let data = client
        .query::<_, crate::queries::list_contests::ResponseData>(
            &crate::queries::ListContests::build_query(crate::queries::list_contests::Variables {
                detailed: true,
            }),
        )
        .expect("network error")
        .into_result()
        .expect("request rejected");
    let mut target_contest = None;
    for contest in data.contests {
        if contest.id == contest_name {
            target_contest = Some(contest);
            break;
        }
    }
    let contest = target_contest.unwrap_or_else(|| {
        eprintln!("contest {} not found", contest_name);
        exit(1);
    });

    for problem in contest.problems {
        if problem.id == problem_code {
            return (contest.id, problem.id);
        }
    }
    eprintln!("problem {} not found", problem_code);
    exit(1);
}
struct Run {
    run_id: i64,
    current_score: i64,
    current_test: i64,
}

impl Run {
    fn new(run_id: i64) -> Run {
        Run {
            run_id,
            current_score: 0,
            current_test: 0,
        }
    }

    fn poll(&mut self, client: &Client) -> Option<crate::queries::view_run::ViewRunFindRun> {
        let poll_lsu_vars = crate::queries::view_run::Variables {
            run_id: self.run_id,
        };
        let resp = client
            .query::<_, crate::queries::view_run::ResponseData>(
                &crate::queries::ViewRun::build_query(poll_lsu_vars),
            )
            .expect("network error")
            .into_result();
        let resp = resp
            .expect("poll LSU failed")
            .find_run
            .expect("run not found");
        let lsu = &resp.live_status_update;
        if let Some(ct) = &lsu.current_test {
            self.current_test = *ct;
        }
        if let Some(ls) = &lsu.live_score {
            self.current_score = *ls;
        }
        println!(
            "score = {}, running on test {}",
            self.current_score, self.current_test
        );
        if lsu.finish {
            println!("judging finished");
            return Some(resp);
        }
        None
    }
}
fn make_submit(client: &Client, contest: &str, problem: &str, code: &str, toolchain: &str) -> Run {
    let vars = crate::queries::submit::Variables {
        toolchain: toolchain.to_string(),
        code: code.to_string(),
        problem: problem.to_string(),
        contest: contest.to_string(),
    };

    let resp = client
        .query::<_, crate::queries::submit::ResponseData>(&crate::queries::Submit::build_query(
            vars,
        ))
        .expect("network error")
        .into_result();
    let run_id = resp.expect("submit failed").submit_simple.id;
    println!("submitted: id={}", run_id);
    Run::new(run_id)
}

pub fn exec(opt: Opt, params: &super::CommonParams) -> Value {
    let data = std::fs::read(&opt.filename).expect("Couldn't read file");
    let code = base64::encode(&data);

    let toolchain = resolve_toolchain(&params.client, &opt.toolchain);
    let (contest, problem) = resolve_problem(&params.client, &opt.contest, &opt.problem);
    let mut runs = Vec::new();
    for _ in 0..opt.count {
        let run = make_submit(&params.client, &contest, &problem, &code, &toolchain);
        runs.push(run);
    }
    while !runs.is_empty() {
        let mut nruns = Vec::new();
        for mut run in runs {
            if let Some(final_results) = run.poll(&params.client) {
                println!(
                    "status: {}({}), score: {}",
                    final_results
                        .status
                        .as_ref()
                        .map(|s| s.kind.to_string())
                        .unwrap_or_else(|| "<missing>".to_string()),
                    final_results
                        .status
                        .as_ref()
                        .map(|s| s.code.to_string())
                        .unwrap_or_else(|| "<missing>".to_string()),
                    final_results
                        .score
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| "<missing>".to_string())
                );
            } else {
                nruns.push(run);
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        runs = nruns;
    }
    serde_json::Value::Null
}
