mod invoker;
mod object;
mod simple_invoker;

use amqp::{Basic, Session, Table};
use invoker_task::*;
use std::path::PathBuf;

const AMQP_URL: &str = "amqp://localhost//";
const QUEUE_NAME: &str = "jjs_invoker";

fn handle_judge_task(task: InvokeRequest, cfg: &config::Config, db: &db::Db) {
    let file_path = PathBuf::from(format!(
        "{}/var/jjs/submits/{}",
        cfg.sysroot.to_string_lossy(),
        task.submission_id
    ));
    if !file_path.exists() {
        println!("file not exists");
        return;
    }

    let toolchain_name = db.submissions.find_by_id(task.submission_id).toolchain;

    let submission = object::Submission::from_file_path(&file_path, &toolchain_name);

    let status = simple_invoker::judge(submission, cfg);

    println!("Judging result: {:#?}", status);
}

fn handle_task(task: Request, cfg: &config::Config, db: &db::Db) {
    match task {
        Request::Print(req) => {
            println!("print: {}", req.data);
        }
        Request::Exit(_req) => {
            println!("exiting.");
            std::process::exit(0);
        }
        Request::Invoke(req) => {
            println!("judging {}", &req.submission_id);
            handle_judge_task(req, cfg, db);
        }
        Request::Noop(_req) => {}
    }
}

fn main() {
    let config = config::get_config();
    let db_conn = db_conn::connect_pg();
    println!("{:#?}", config);
    let mut session = Session::open_url(AMQP_URL).unwrap();
    let mut channel = session.open_channel(1).unwrap();
    let queue_declare =
        channel.queue_declare(QUEUE_NAME, false, true, false, false, false, Table::new());
    queue_declare.unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        for get_result in channel.basic_get(QUEUE_NAME, true) {
            let body = String::from_utf8(get_result.body.clone()).unwrap();
            println!("got query: {}", body);
            let request: invoker_task::Request = serde_json::from_str(&body).unwrap();
            handle_task(request, &config, &db_conn);
        }
    }
}
