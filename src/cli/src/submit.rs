use client::{Api as _, ApiClient};
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

async fn resolve_toolchain(client: &ApiClient, name: &str) -> String {
    let list = client
        .list_toolchains()
        .await
        .expect("Couldn't get toolchain information");
    for tc in list {
        if tc.id == name {
            return tc.id;
        }
    }
    panic!("Couldn't find toolchain {}", name);
}

async fn resolve_problem(
    client: &ApiClient,
    contest_name: &str,
    problem_code: &str,
) -> (String, String) {
    let data = client.list_contests().await.unwrap();
    let mut target_contest = None;
    for contest in data {
        if contest.id == contest_name {
            target_contest = Some(contest);
            break;
        }
    }
    let contest = target_contest.unwrap_or_else(|| {
        eprintln!("contest {} not found", contest_name);
        exit(1);
    });

    let problems = client.list_contest_problems(&contest.id).await.unwrap();

    for problem in problems {
        if problem.rel_name == problem_code {
            return (contest.id, problem.rel_name);
        }
    }
    eprintln!("problem {} not found", problem_code);
    exit(1);
}
struct Run {
    run_id: i32,
    current_score: i64,
    current_test: i64,
}

impl Run {
    fn new(run_id: i32) -> Run {
        Run {
            run_id,
            current_score: 0,
            current_test: 0,
        }
    }

    async fn poll(&mut self, client: &ApiClient) -> Option<client::models::Run> {
        let lsu = client.get_run_live_status(self.run_id).await.unwrap();
        if let Some(ct) = &lsu.current_test {
            self.current_test = *ct as i64;
        }
        if let Some(ls) = &lsu.live_score {
            self.current_score = *ls as i64;
        }
        println!(
            "score = {}, running on test {}",
            self.current_score, self.current_test
        );
        if lsu.finish {
            println!("judging finished");
            return Some(client.get_run(self.run_id).await.unwrap());
        }
        None
    }
}
async fn make_submit(
    client: &ApiClient,
    contest: &str,
    problem: &str,
    code: &str,
    toolchain: &str,
) -> Run {
    let params = client::models::RunSimpleSubmitParams {
        toolchain: toolchain.to_string(),
        code: code.to_string(),
        problem: problem.to_string(),
        contest: contest.to_string(),
    };

    let resp = client.submit_run(params).await;
    let run_id = resp.expect("submit failed").id;
    println!("submitted: id={}", run_id);
    Run::new(run_id)
}

pub async fn exec(opt: Opt, params: &super::CommonParams) -> Value {
    let data = std::fs::read(&opt.filename).expect("Couldn't read file");
    let code = base64::encode(&data);

    let toolchain = resolve_toolchain(&params.client, &opt.toolchain).await;
    let (contest, problem) = resolve_problem(&params.client, &opt.contest, &opt.problem).await;
    let mut runs = Vec::new();
    for _ in 0..opt.count {
        let run = make_submit(&params.client, &contest, &problem, &code, &toolchain).await;
        runs.push(run);
    }
    while !runs.is_empty() {
        let mut nruns = Vec::new();
        for mut run in runs {
            if let Some(final_results) = run.poll(&params.client).await {
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
