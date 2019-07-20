#![feature(option_flattening)]

mod compiler;
mod inter_api;
mod invoke_context;
mod invoker;
mod judge;
mod valuer;

use cfg_if::cfg_if;
use db::schema::{InvokationRequest, Submission};
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

    impl From<std::io::Error> for Error {
        fn from(x: std::io::Error) -> Error {
            Error::Io {
                source: x,
                backtrace: Backtrace::new(),
            }
        }
    }
}

pub(crate) use err::{Error, StringError};
use std::path::Path;

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
    pub id: u32,
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
    request: &InvokationRequest,
) {
    use db::schema::submissions::dsl::*;
    let target = submissions.filter(id.eq(submission_id as i32));
    let subm_patch = db::schema::SubmissionPatch {
        state: Some(db::schema::SubmissionState::Done),
        status_code: Some(outcome.status.code.to_string()),
        status_kind: Some(outcome.status.kind.to_string()),
        score: Some(outcome.score as i32),
        rejudge_id: Some(request.invoke_revision() as i32),
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

    fn try_get_task(&self) -> Option<InvokationRequest> {
        use db::schema::invokation_requests::dsl::*;
        let res: Option<InvokationRequest> = self
            .db_conn
            .transaction::<_, diesel::result::Error, _>(|| {
                let waiting_submission = invokation_requests
                    .limit(1)
                    .load::<InvokationRequest>(&self.db_conn)
                    .expect("db error");
                let waiting_submission = waiting_submission.into_iter().next();
                match waiting_submission {
                    Some(s) => {
                        diesel::delete(invokation_requests)
                            .filter(id.eq(s.id))
                            .execute(&self.db_conn)
                            .expect("db error");

                        Ok(Some(s))
                    }
                    None => Ok(None),
                }
            })
            .ok()
            .flatten();

        res
    }

    /// called by every thread
    fn thread_loop(&self, should_run: Arc<AtomicBool>) {
        loop {
            if !should_run.load(sync::atomic::Ordering::SeqCst) {
                break;
            }

            let inv_req = match self.try_get_task() {
                Some(s) => s,
                None => {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    continue;
                }
            };
            let submission_id = inv_req.submission_id();
            match self.process_task(inv_req) {
                Ok(_) => {}
                Err(err) => {
                    error!(self.logger, "Invokation fault"; "submission" => submission_id, "message" => %err, "message-detailed" => ?err);
                }
            }
        }
    }

    fn process_task(&self, inv_req: InvokationRequest) -> Result<(), Error> {
        let req = self.fetch_submission_info(&inv_req)?;
        let submission_id = req.submission.id;
        let outcome = self.process_invoke_request(&req);
        submission_set_judge_outcome(&self.db_conn, submission_id, outcome, &inv_req);
        self.copy_invokation_data_dir_to_shared_fs(&req.work_dir.path(), submission_id, req.id)?;
        Ok(())
    }

    fn copy_invokation_data_dir_to_shared_fs(
        &self,
        temp_path: &Path,
        run_id: u32,
        inv_id: u32,
    ) -> Result<(), Error> {
        let target_dir = self
            .config
            .sysroot
            .join("var/submissions")
            .join(format!("s-{}", run_id))
            .join(format!("i-{}", inv_id));
        std::fs::create_dir_all(&target_dir)?;
        let from = vec![temp_path];
        fs_extra::copy_items(&from, &target_dir, &fs_extra::dir::CopyOptions::new())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }

    fn process_invoke_request(&self, request: &InvokeRequest) -> invoker::InvokeOutcome {
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
        let status = invoker.invoke().unwrap_or_else(|err| {
            let cause = err
                .source()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "<missing>".to_string());
            let backtrace = err
                .backtrace()
                .map(|bt| bt.to_string())
                .unwrap_or_else(|| "<not captured>".to_string());
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
        status
    }

    /// This functions queries all related data about submission and returns JudgeRequest
    fn fetch_submission_info(
        &self,
        db_inv_req: &InvokationRequest,
    ) -> Result<InvokeRequest, Error> {
        let db_submission;
        {
            use db::schema::submissions::dsl::*;
            db_submission = submissions
                .filter(id.eq(db_inv_req.submission_id() as i32))
                .load::<Submission>(&self.db_conn)
                .unwrap()
                .into_iter()
                .next()
                .unwrap();
        }

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
            id: db_inv_req.id(),
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
