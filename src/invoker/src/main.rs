mod compiler;
mod inter_api;
mod invoke_context;
mod invoke_env;
mod invoke_util;
mod invoker;
mod judge;
mod judge_log;
mod os_util;
mod valuer;

use crate::{invoke_context::MainInvokeContext, invoke_env::InvokeEnv};
use anyhow::{bail, Context};
use cfg_if::cfg_if;
use db::schema::InvocationRequest;
use invoker::Invoker;
use invoker_api::InvokeTask;
use slog_scope::{debug, error};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::exit,
    sync::{self, atomic::AtomicBool, Arc},
};

/// Secondary information, used for various interpolations
#[derive(Debug)]
pub(crate) struct RunProps {
    pub metadata: HashMap<String, String>,
    pub id: i32,
}

/// Submission information, sufficient for judging
#[derive(Debug)]
pub(crate) struct RunInfo {
    pub toolchain_cfg: cfg::Toolchain,
    pub problem_cfg: cfg::Problem,
    pub problem_data: pom::Problem,
    /// Directory for general files (source, build, invlog)
    pub root_dir: PathBuf,
    pub props: RunProps,
}

#[derive(Debug)]
/// All invoker-related data, that will be passed to Invoker
pub(crate) struct InvokeRequest {
    pub run: RunInfo,
    /// Temporary directory
    pub work_dir: tempfile::TempDir,
    pub revision: u32,
    pub live_webhook: Option<String>,
}

cfg_if! {
if #[cfg(target_os="linux")] {
    fn check_system() -> anyhow::Result<()> {
        if let Some(err) = minion::linux_check_environment() {
            bail!("system configuration problem: {}", err);
        }
        Ok(())
    }
} else {
    fn check_system() -> anyhow::Result<()> {
        true
    }
}
}

fn set_run_judge_outcome(
    conn: &dyn db::DbConn,
    run_id: i32,
    outcome: invoker::InvokeOutcome,
    request: &InvokeTask,
) -> anyhow::Result<()> {
    let run_patch = db::schema::RunPatch {
        status_code: Some(outcome.status.code.to_string()),
        status_kind: Some(outcome.status.kind.to_string()),
        score: Some(outcome.score as i32),
        rejudge_id: Some(request.revision as i32),
    };

    conn.run_update(run_id, run_patch)
        .context("failed to update run in db")?;
    Ok(())
}

struct Server {
    config: cfg::Config,
    db_conn: Box<dyn db::DbConn>,
    backend: Box<dyn minion::Backend>,
}

impl Server {
    fn serve_forever(&self) -> anyhow::Result<()> {
        let should_run = sync::Arc::new(sync::atomic::AtomicBool::new(true));
        {
            let should_run = sync::Arc::clone(&should_run);
            ctrlc::set_handler(move || {
                should_run.store(false, sync::atomic::Ordering::SeqCst);
            })
            .context("failed to set ctrl-c handler")?;
        }
        //TODO: start multiple threads
        self.thread_loop(Arc::clone(&should_run));
        Ok(())
    }

    fn try_get_task(&self) -> Option<InvokeTask> {
        let res: Option<InvocationRequest> = self
            .db_conn
            .inv_req_pop() // TODO handle error
            .ok()
            .flatten();

        res.map(|t| t.invoke_task)
    }

    /// called by every thread
    fn thread_loop(&self, should_run: Arc<AtomicBool>) {
        loop {
            if !should_run.load(sync::atomic::Ordering::SeqCst) {
                break;
            }

            let invoke_task = match self.try_get_task() {
                Some(it) => it,
                None => {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    continue;
                }
            };
            let run_id = invoke_task.run_id;
            match self.process_task(invoke_task) {
                Ok(_) => (),
                Err(err) => {
                    error!("Invocation fault"; "run" => run_id, "message" => %err, "message-detailed" => ?err);
                }
            }
        }
    }

    fn process_task(&self, invoke_task: InvokeTask) -> anyhow::Result<()> {
        let req = self
            .fetch_run_info(&invoke_task)
            .context("failed to fetch run information")?;
        let run_id = req.run.props.id;
        let outcome = self.process_invoke_request(&req);
        set_run_judge_outcome(&*self.db_conn, run_id, outcome, &invoke_task)
            .context("failed to save judge outcome")?;
        self.copy_invocation_data_dir_to_shared_fs(&req.work_dir.path(), run_id, req.revision)
            .context("failed to update shared fs")?;
        Ok(())
    }

    fn copy_invocation_data_dir_to_shared_fs(
        &self,
        temp_path: &Path,
        run_id: i32,
        revision: u32,
    ) -> anyhow::Result<()> {
        let target_dir = self
            .config
            .sysroot
            .join("var/submissions")
            .join(format!("s-{}", run_id))
            .join(format!("i-{}", revision));
        std::fs::create_dir_all(&target_dir).context("failed to create target dir")?;
        let from: Result<Vec<_>, _> = std::fs::read_dir(temp_path)
            .context("failed to list source directory")?
            .map(|x| x.map(|y| y.path()))
            .collect();
        fs_extra::copy_items(&from?, &target_dir, &fs_extra::dir::CopyOptions::new())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            .context("failed to copy")?;
        Ok(())
    }

    fn process_invoke_request(&self, request: &InvokeRequest) -> invoker::InvokeOutcome {
        let invoke_data = InvokeEnv {
            minion_backend: &*self.backend,
            cfg: &self.config,
            problem_cfg: &request.run.problem_cfg,
            toolchain_cfg: &request.run.toolchain_cfg,
            problem_data: &request.run.problem_data,
            run_props: &request.run.props,
        };

        let invoke_ctx = MainInvokeContext { env: invoke_data };
        let invoker = Invoker::new(&invoke_ctx, request);
        debug!("Executing invoker request"; "request" => ?request, "run" => ?request.run.props.id, "workdir" => ?request.work_dir.path().display());
        let status = invoker.invoke().unwrap_or_else(|err| {
            error!("Judge fault: {:?}", err);
            let st = invoker_api::Status {
                kind: invoker_api::StatusKind::InternalError,
                code: invoker_api::status_codes::JUDGE_FAULT.to_string(),
            };
            invoker::InvokeOutcome {
                status: st,
                score: 0,
            }
        });

        debug!("Judging finished"; "outcome" => ?status, "run-id" => ?request.run.props.id);
        status
    }

    /// This functions queries all related data about run and returns InvokeRequest
    ///
    /// InvokeTask is not single source of trust, and some information needs to be taken from
    /// database.
    /// But InvokeRequest **is** SSoT, and invoker engine is completely isolated from other
    /// components.
    fn fetch_run_info(&self, invoke_task: &InvokeTask) -> anyhow::Result<InvokeRequest> {
        let db_run = self.db_conn.run_load(invoke_task.run_id as i32)?;

        let run_root = self.config.sysroot.join("var/submissions");
        let run_root = run_root.join(&format!("s-{}", db_run.id));

        let mut run_metadata = HashMap::new();
        let judge_time = {
            let time = chrono::prelude::Utc::now();
            time.format("%Y-%m-%d %H:%M:%S").to_string()
        };
        run_metadata.insert("JudgeTimeUtc".to_string(), judge_time);

        let prob_name = &db_run.problem_id;

        let problem_manifest_path = self
            .config
            .sysroot
            .join("var/problems")
            .join(&prob_name)
            .join("manifest.json");

        let reader = std::io::BufReader::new(
            fs::File::open(problem_manifest_path).context("failed to read problem manifest")?,
        );

        let problem_data: pom::Problem =
            serde_json::from_reader(reader).context("failed to parse problem manifest")?;

        let toolchain_cfg = self
            .config
            .find_toolchain(&db_run.toolchain_id)
            .ok_or_else(|| anyhow::anyhow!("toolchain {} not found", &db_run.toolchain_id))?;

        let problem_cfg = self
            .config
            .find_problem(&db_run.problem_id)
            .ok_or_else(|| anyhow::anyhow!("problem {} not found", &db_run.problem_id))?;

        let run_props = RunProps {
            metadata: run_metadata,
            id: db_run.id,
        };

        let run = RunInfo {
            root_dir: run_root,
            props: run_props,
            toolchain_cfg: toolchain_cfg.clone(),
            problem_data,
            problem_cfg: problem_cfg.clone(),
        };

        let req = InvokeRequest {
            run,
            work_dir: tempfile::TempDir::new().context("failed to get temp dir")?,
            revision: invoke_task.revision,
            live_webhook: invoke_task.status_update_callback.clone(),
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
    if atty::is(atty::Stream::Stderr) {
        install_color_backtrace();
    }
    util::log::setup();
    util::daemon_startup_sleep();
    util::wait::wait();

    let config = cfg::get_config();
    let db_conn = match db::connect_env() {
        Ok(db_conn) => db_conn,
        Err(e) => {
            eprintln!("Startup error: failed connect to database: {}", e);
            exit(1);
        }
    };

    match check_system() {
        Ok(()) => debug!("system check passed"),
        Err(err) => {
            eprintln!("system configuration problem: {}", err);
            return;
        }
    }
    let backend = minion::setup();

    let invoker = Server {
        config,
        db_conn,
        backend,
    };

    util::daemon_notify_ready();

    if let Err(e) = invoker.serve_forever() {
        eprintln!("{:?}", e);
    }
}
