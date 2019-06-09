mod judge;

use cfg::Config;
use cfg_if::cfg_if;
use db::schema::{Submission, SubmissionState};
use diesel::{pg::PgConnection, prelude::*};
use judge::Judger;
use slog::*;
use std::{collections::HashMap, fs, sync};

#[derive(Debug)]
pub struct SubmissionProps {
    pub toolchain: String,
}

#[derive(Debug)]
pub struct JudgeRequest {
    pub submission_root: String,
    pub submission_metadata: HashMap<String, String>,
    pub submission_props: SubmissionProps,
    pub problem: pom::Problem,
    pub judging_id: u32,
}

cfg_if! {
if #[cfg(target_os="linux")] {
    fn check_system() -> bool {
        if let Some(err) = minion::linux_check_environment() {
            eprintln!("system configuration issue: {}", err);
            return false;
        }
        true
    }
} else {
    fn check_system() -> bool {
        true
    }
}
}

fn handle_judge_task(
    request: JudgeRequest,
    cfg: &Config,
    conn: &PgConnection,
    logger: &slog::Logger,
    submission_id: u32,
) {
    use db::schema::submissions::dsl::*;

    let judger = Judger {
        cfg,
        logger,
        request: &request,
        problem: &request.problem,
    };
    slog::debug!(logger, "Executing judge request"; "request" => ?request, "submission" => submission_id);
    let judging_status = judger.judge();

    let target = submissions.filter(id.eq(submission_id as i32));
    let subm_patch = db::schema::SubmissionPatch {
        state: Some(db::schema::SubmissionState::Done),
        status_code: Some(judging_status.code.to_string()),
        status_kind: Some(judging_status.kind.to_string()),
        judge_revision: Some(request.judging_id as i32),
    };
    diesel::update(target)
        .set(subm_patch)
        .execute(conn)
        .expect("Db query failed");
    debug!(logger, "judging finished"; "outcome" => ?judging_status);
}

fn main() {
    use db::schema::submissions::dsl::*;
    dotenv::dotenv().ok();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let root = Logger::root(drain, o!("app"=>"jjs:invoker"));

    info!(root, "starting");

    let config = cfg::get_config();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL - must contain postgres URL");
    let db_conn = diesel::pg::PgConnection::establish(db_url.as_str())
        .unwrap_or_else(|_e| panic!("Couldn't connect to {}", db_url));
    let should_run = sync::Arc::new(sync::atomic::AtomicBool::new(true));
    {
        let should_run = sync::Arc::clone(&should_run);
        ctrlc::set_handler(move || {
            should_run.store(false, sync::atomic::Ordering::SeqCst);
        })
        .unwrap();
    }

    if check_system() {
        debug!(root, "system check passed")
    } else {
        return;
    }

    loop {
        if !should_run.load(sync::atomic::Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        let waiting_submission = submissions
            .filter(state.eq(SubmissionState::WaitInvoke))
            .limit(1)
            .load::<Submission>(&db_conn)
            .expect("db error");
        let waiting_submission = waiting_submission.get(0);
        let waiting_submission = match waiting_submission {
            Some(s) => s.clone(),
            None => continue,
        };

        let submission_root = format!(
            "{}/var/submissions/s-{}",
            config.sysroot,
            waiting_submission.id()
        );

        let mut submission_metadata = HashMap::new();
        submission_metadata.insert("Id".to_string(), waiting_submission.id().to_string());

        let prob_name = &waiting_submission.problem_name;

        let problem_manifest_path = format!(
            "{}/var/problems/{}/manifest.json",
            config.sysroot, &prob_name
        );
        let problem: pom::Problem =
            serde_json::from_reader(fs::File::open(problem_manifest_path).unwrap()).unwrap();

        let req = JudgeRequest {
            submission_root,
            submission_metadata,
            judging_id: (waiting_submission.judge_revision + 1) as u32,
            submission_props: SubmissionProps {
                toolchain: waiting_submission.toolchain.clone(),
            },
            problem,
        };
        handle_judge_task(req, &config, &db_conn, &root, waiting_submission.id());
    }
}
