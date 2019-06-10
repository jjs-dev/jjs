use frontend_api::*;
use std::process::exit;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    /// problem code
    problem: String,
    toolchain: String,
    filename: String,
    #[structopt(long = "token", short = "t", default_value = "dev:root")]
    token: String,
}

fn resolve_toolchain(client: &Client, name: &str) -> u32 {
    let res: Result<Vec<ToolchainInformation>, CommonError> =
        client.toolchains_list(&()).expect("network error");
    let res = res.expect("Couldn't get toolchain information");
    for tc in res {
        if tc.name == name {
            return tc.id;
        }
    }
    panic!("Couldn't find toolchain {}", name);
}

fn resolve_problem(
    client: &Client,
    contest_name: &str,
    problem_code: &str,
) -> (frontend_api::ContestId, frontend_api::ProblemCode) {
    let contests = client
        .contests_list(&())
        .expect("network error")
        .expect("request rejected");
    let mut contest_id = None;
    for contest in contests {
        if contest.name == contest_name {
            contest_id = Some(contest.name);
            break;
        }
    }
    let contest_id = contest_id.unwrap_or_else(|| {
        eprintln!("contest {} not found", contest_name);
        exit(1);
    });

    let contest_info = client
        .contests_describe(&contest_id)
        .expect("network error")
        .expect("request rejected");
    for problem in contest_info.problems.unwrap() {
        if problem.code == problem_code {
            return (contest_id, problem.code);
        }
    }
    eprintln!("problem {} not found", problem_code);
    exit(1);
}

fn main() {
    let opt: Opt = Opt::from_args();
    let token = opt.token.clone();
    let data = std::fs::read(&opt.filename).expect("Couldn't read file");
    let data = base64::encode(&data);
    let client = Client::new("http://localhost:1779".to_string(), token);
    let tc_id = resolve_toolchain(&client, &opt.toolchain);
    let (contest, problem) = resolve_problem(&client, "TODO", &opt.problem);
    let query = SubmissionSendParams {
        toolchain: tc_id,
        code: data,
        problem,
        contest,
    };
    let resp = client.submissions_send(&query).expect("network error");
    let resp = resp.expect("submit failed");
    println!("submitted successfully, id={}", resp);
}
