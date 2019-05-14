use frontend_api::*;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    toolchain: String,
    filename: String,
    #[structopt(long="token", short="t", default_value="dev:root")]
    token: String,
}

fn resolve_toolchain(client: &Client, name: &str) -> u32 {
    let res: Result<Vec<ToolchainInformation>, CommonError> = client
        .toolchains_list(&())
        .expect("network error");
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
    let token = opt.token.clone();
    let data = std::fs::read(&opt.filename).expect("Couldn't read file");
    let data = base64::encode(&data);
    let client = Client::new("http://localhost:1779".to_string(), token);
    let tc_id = resolve_toolchain(&client, &opt.toolchain);
    let query = SubmissionSendParams {
        toolchain: tc_id,
        code: data,
    };
    let resp = client.submissions_send(&query).expect("network error");
    let resp = resp.expect("submit failed");
    println!("submitted successfully, id={}", resp);
}
