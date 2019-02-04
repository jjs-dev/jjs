mod invoker;
mod simple_invoker;
use std::sync;

use slog::*;

struct InvokeRequest {
    submission: domain::Submission,
}

fn handle_judge_task(task: InvokeRequest, cfg: &config::Config, db: &db::Db) {
    //let file_path = PathBuf::from(format!(
    //    "{}/var/jjs/submits/{}",
    //    cfg.sysroot,
    //    task.submission.id
    //));

    //let toolchain_name = db.submissions.find_by_id(task.submission.id).toolchain;

    let submission = task.submission.clone();

    let status = simple_invoker::judge(&submission, cfg);

    db.submissions
        .update_submission_state(&task.submission, domain::SubmissionState::Done);

    println!("Judging result: {:#?}", status);
}

fn main() {
    dotenv::dotenv().ok();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let root = Logger::root(drain, o!("app"=>"jjs:invoker"));

    info!(root, "starting");

    let config = config::get_config();
    let db_conn = db_conn::connect_pg();
    //println!("{:#?}", config);
    let should_run = sync::Arc::new(sync::atomic::AtomicBool::new(true));
    {
        let should_run = sync::Arc::clone(&should_run);
        ctrlc::set_handler(move || {
            should_run.store(false, sync::atomic::Ordering::SeqCst);
        })
        .unwrap();
    }
    loop {
        if !should_run.load(sync::atomic::Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        let waiting_submission = db_conn.submissions.find_next_waiting();
        let waiting_submission = match waiting_submission {
            Some(s) => s,
            None => continue,
        };
        let ivr = InvokeRequest {
            submission: waiting_submission,
        };
        handle_judge_task(ivr, &config, &db_conn);
    }
}
