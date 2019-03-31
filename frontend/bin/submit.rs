use frontend_api::*;
use serde_json;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    toolchain: String,
    filename: String,
}

fn resolve_toolchain(name: &str) -> u32 {
    let res: Result<Vec<ToolchainInformation>, CommonError> =
        reqwest::get("http://localhost:1779/toolchains/list")
            .expect("network error")
            .json()
            .expect("parse error");
    let res = res.expect("Couldn't get toolchain information");
    for tc in res {
        if tc.name == name {
            return tc.id;
        }
    }
    panic!("Couldn't find toolchain {}", name);
}

fn main() {
    let opt: Opt = Opt::from_args();
    let token = "dev:root".to_string();
    let data = std::fs::read(&opt.filename).expect("Couldn't read file");
    let data = base64::encode(&data);
    let tc_id = resolve_toolchain(&opt.toolchain);
    let query = SubmitDeclaration {
        toolchain: tc_id,
        code: data,
    };
    let resp: Result<SubmissionId, SubmitError> = reqwest::Client::new()
        .post("http://localhost:1779/submissions/send")
        .header("X-Jjs-Auth", token)
        .body(serde_json::to_string(&query).unwrap())
        .send()
        .expect("network error")
        .json()
        .expect("parse error");
    let resp = resp.expect("submit failed");
    println!("submitted successfully, id={}", resp);
}
