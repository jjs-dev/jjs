mod compiler;
mod inter_api;
mod invoke_context;
mod invoker;
mod judge;
mod valuer;

use cfg_if::cfg_if;
use db::schema::{Submission, SubmissionState};
use diesel::{pg::PgConnection, prelude::*};
use invoker::{InvokeContext, Invoker};
use slog::{debug, error, info, o, Drain, Logger};
use snafu::ResultExt;
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{self, atomic::AtomicBool, Arc},
};

pub(crate) mod err {
    use snafu::{Backtrace, Snafu};
    use std::fmt::{self, Debug, Display, Formatter};

    pub struct StringError(pub String);

    impl Display for StringError {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            Display::fmt(&self.0, f)
        }
    }

    impl Debug for StringError {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            Debug::fmt(&self.0, f)
        }
    }

    impl std::error::Error for StringError {}

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    pub enum Error {
        Minion {
            source: minion::Error,
            backtrace: Backtrace,
        },
        Io {
            source: std::io::Error,
            backtrace: Backtrace,
        },
        /// Usually, these errors occur if system was given malformed configuration
        /// For example, if interpolation string is bad
        #[snafu(display("Bad config: {}", inner))]
        BadConfig {
            backtrace: Backtrace,
            inner: Box<dyn std::error::Error + Send + Sync + 'static>,
        },
        #[snafu(display("Error: {}", inner))]
        Other {
            backtrace: Backtrace,
            inner: Box<dyn std::error::Error + Send + Sync + 'static>,
        },
    }
}

pub(crate) use err::{Error, StringError};

#[derive(Debug)]
/// Submission information, sufficient for judging
pub(crate) struct SubmissionInfo {
    pub toolchain: cfg::Toolchain,
    /// Directory for general files (source, build, invlog)
    pub root_dir: PathBuf,
    pub metadata: HashMap<String, String>,
    pub id: u32,
}

#[derive(Debug)]
/// All invoker-related data, that will be passed to Invoker
pub(crate) struct InvokeRequest {
    pub submission: SubmissionInfo,
    pub problem: pom::Problem,
    /// Temporary directory
    pub work_dir: tempfile::TempDir,
}

cfg_if! {
if #[cfg(target_os="linux")] {
    fn check_system() -> bool {
        if let Some(err) = minion::linux_check_environment() {
            eprintln!("system configuration problem: {}", err);
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

fn submission_set_judge_outcome(
    conn: &PgConnection,
    submission_id: u32,
    outcome: invoker::InvokeOutcome,
) {
    use db::schema::submissions::dsl::*;
    let target = submissions.filter(id.eq(submission_id as i32));
    let subm_patch = db::schema::SubmissionPatch {
        state: Some(db::schema::SubmissionState::Done),
        status_code: Some(outcome.status.code.to_string()),
        status_kind: Some(outcome.status.kind.to_string()),
        score: Some(outcome.score as i32),
    };
    diesel::update(target)
        .set(subm_patch)
        .execute(conn)
        .expect("Db query failed");
}

struct Server {
    config: cfg::Config,
    logger: Logger,
    db_conn: PgConnection,
    backend: Box<dyn minion::Backend>,
}

impl Server {
    fn serve_forever(&self) {
        let should_run = sync::Arc::new(sync::atomic::AtomicBool::new(true));
        {
            let should_run = sync::Arc::clone(&should_run);
            ctrlc::set_handler(move || {
                should_run.store(false, sync::atomic::Ordering::SeqCst);
            })
                .unwrap();
        }
        //TODO: start multiple threads
        self.thread_loop(Arc::clone(&should_run));
    }

    fn try_get_task(&self) -> Option<Submission> {
        use db::schema::submissions::dsl::*;
        let waiting_submission = submissions
            .filter(state.eq(SubmissionState::WaitInvoke))
            .limit(1)
            .load::<Submission>(&self.db_conn)
            .expect("db error");
        let waiting_submission = waiting_submission.get(0);
        match waiting_submission {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    /// called by every thread
    fn thread_loop(&self, should_run: Arc<AtomicBool>) {
        loop {
            if !should_run.load(sync::atomic::Ordering::SeqCst) {
                break;
            }

            let submission = match self.try_get_task() {
                Some(s) => s,
                None => {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    continue;
                }
            };
            let submission_id = submission.id();
            match self.process_task(submission) {
                Ok(_) => {}
                Err(err) => {
                    error!(self.logger, "Invokation fault"; "submission" => submission_id, "message" => %err, "message-detailed" => ?err);
                }
            }
        }
    }

    fn process_task(&self, submission: Submission) -> Result<(), Error> {
        let req = self.fetch_submission_info(submission)?;
        let submission_id = req.submission.id;
        let outcome = self.process_invoke_request(req);
        submission_set_judge_outcome(&self.db_conn, submission_id, outcome);
        Ok(())
    }

    fn process_invoke_request(&self, request: InvokeRequest) -> invoker::InvokeOutcome {
        use snafu::ErrorCompat;
        use std::error::Error;
        let invoke_ctx = InvokeContext {
            minion_backend: &*self.backend,
            cfg: &self.config,
            logger: &self.logger,
            req: &request,
        };
        let invoker = Invoker::new(invoke_ctx);
        debug!(self.logger, "Executing invoker request"; "request" => ?request, "submission" => ?request.submission.id, "workdir" => ?request.work_dir.path().display());
        let status =
            invoker
                .invoke()
                .unwrap_or_else(|err| {
                    let cause = err.source().map(|e| e.to_string()).unwrap_or_else(|| "<missing>".to_string());
                    let backtrace = err.backtrace().map(|bt| bt.to_string()).unwrap_or_else(|| "<not captured>".to_string());
                    error!(self.logger, "Judge fault: {}", err; "backtrace" => backtrace, "cause" => cause);
                    let st = invoker_api::Status {
                        kind: invoker_api::StatusKind::InternalError,
                        code: invoker_api::status_codes::JUDGE_FAULT.to_string(),
                    };
                    invoker::InvokeOutcome {
                        status: st,
                        score: 0,
                    }
                });

        debug!(self.logger, "Judging finished"; "outcome" => ?status, "submission" => ?request.submission.id);
        request.work_dir.into_path();
        status
    }

    /// This functions queries all related data about submission and returns JudgeRequest
    fn fetch_submission_info(&self, db_submission: Submission) -> Result<InvokeRequest, Error> {
        let submission_root = self.config.sysroot.join("var/submissions");
        let submission_root = submission_root.join(&format!("s-{}", db_submission.id()));

        let mut submission_metadata = HashMap::new();
        let judge_time = {
            let time = chrono::prelude::Utc::now();
            time.format("%Y-%m-%d %H:%M:%S").to_string()
        };
        submission_metadata.insert("JudgeTimeUtc".to_string(), judge_time);

        let prob_name = &db_submission.problem_name;

        let problem_manifest_path = self
            .config
            .sysroot
            .join("var/problems")
            .join(&prob_name)
            .join("manifest.json");

        let reader =
            std::io::BufReader::new(fs::File::open(problem_manifest_path).context(err::Io)?);

        let problem: pom::Problem = serde_json::from_reader(reader).map_err(|e| Error::Other {
            backtrace: Default::default(),
            inner: Box::new(e),
        })?;

        let toolchain =
            self.config
                .find_toolchain(&db_submission.toolchain)
                .ok_or(Error::BadConfig {
                    backtrace: Default::default(),
                    inner: Box::new(StringError(format!(
                        "toolchain {} not found",
                        &db_submission.toolchain
                    ))),
                })?;

        let submission = SubmissionInfo {
            root_dir: submission_root,
            metadata: submission_metadata,
            toolchain: toolchain.clone(),
            id: db_submission.id(),
        };

        let req = InvokeRequest {
            submission,
            work_dir: tempfile::TempDir::new().context(err::Io {})?,
            problem,
        };
        Ok(req)
    }
}

cfg_if! {
    if #[cfg(feature = "beautiful_backtrace")] {
        fn install_color_backtrace() {
            color_backtrace::install();
        }
    } else {
        fn install_color_backtrace() {

        }
    }
}

fn main() {
    dotenv::dotenv().ok();
    install_color_backtrace();
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let root = Logger::root(drain, o!("app"=>"jjs:invoker"));

    info!(root, "starting");

    let config = cfg::get_config();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL - must contain postgres URL");
    let db_conn = diesel::pg::PgConnection::establish(db_url.as_str())
        .unwrap_or_else(|_e| panic!("Couldn't connect to {}", db_url));

    if check_system() {
        debug!(root, "system check passed")
    } else {
        return;
    }
    let backend = minion::setup();

    let invoker = Server {
        config,
        logger: root,
        db_conn,
        backend,
    };

    invoker.serve_forever();
}
