mod invoker;
mod object;
mod simple_invoker;
use std::path::PathBuf;

struct InvokeRequest {
    submission: domain::Submission,
}

fn handle_judge_task(task: InvokeRequest, cfg: &config::Config, db: &db::Db) {
    let file_path = PathBuf::from(format!(
        "{}/var/jjs/submits/{}",
        cfg.sysroot.to_string_lossy(),
        task.submission.id
    ));
    if !file_path.exists() {
        println!("file not exists");
        return;
    }

    let toolchain_name = db.submissions.find_by_id(task.submission.id).toolchain;

    let submission = object::Submission::from_file_path(&file_path, &toolchain_name);

    let status = simple_invoker::judge(submission, cfg);

    db.submissions.update_submission_state(&task.submission, "done");

    println!("Judging result: {:#?}", status);
}

fn main() {
    dotenv::dotenv().ok();
    let config = config::get_config();
    let db_conn = db_conn::connect_pg();
    println!("{:#?}", config);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        let waiting_submission = db_conn.submissions.find_next_waiting();
        let waiting_submission = match waiting_submission {
            Some(s) => s,
            None => continue
        };
        let ivr = InvokeRequest {
            submission: waiting_submission,
        };
        handle_judge_task(ivr, &config, &db_conn);
    }
}
