use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone)]
pub(crate) struct Task {
    /// Directory with source files, or path to single file
    pub(crate) src: PathBuf,
    /// Directory for build artifacts
    pub(crate) dest: PathBuf,
    /// Directort for temporary data
    pub(crate) tmp: PathBuf,
}

pub(crate) struct TaskSuccess {
    pub(crate) command: crate::command::Command,
}

#[derive(Debug, Error)]
pub(crate) enum TaskError {
    #[error("child command errored: code {:?}", _0.status.code())]
    ExitCodeNonZero(std::process::Output),
    #[error("io error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("feature not supported: {feature}")]
    FeatureNotSupported { feature: &'static str },
}

impl Task {
    fn multi_file(&self) -> bool {
        self.src.is_dir()
    }
}

#[async_trait::async_trait]
trait CommandExt {
    async fn run(&mut self) -> Result<(), TaskError>;
}

#[async_trait::async_trait]
impl CommandExt for tokio::process::Command {
    async fn run(&mut self) -> Result<(), TaskError> {
        let out = self.output().await?;
        if out.status.success() {
            Ok(())
        } else {
            Err(TaskError::ExitCodeNonZero(out))
        }
    }
}

#[async_trait::async_trait]
pub(crate) trait BuildBackend {
    async fn process_task(&self, task: Task) -> Result<TaskSuccess, TaskError>;
}

/// Ppc-integrated build system
pub(crate) struct Pibs<'a> {
    pub(crate) jjs_dir: &'a Path,
}

impl<'a> Pibs<'a> {
    async fn process_cmake_task(&self, task: Task) -> Result<TaskSuccess, TaskError> {
        //    let cmake_lists = task.src.join("CMakeLists.txt");
        tokio::process::Command::new("cmake")
            .arg("-S")
            .arg(&task.src)
            .arg("-B")
            .arg(&task.tmp)
            .run()
            .await?;

        tokio::process::Command::new("cmake")
            .arg("--build")
            .arg(&task.tmp)
            .run()
            .await?;

        let dst = task.dest.join("bin");
        tokio::fs::copy(task.tmp.join("Out"), &dst).await?;
        let run_cmd = crate::command::Command::new(dst);
        Ok(TaskSuccess { command: run_cmd })
    }
}

#[async_trait::async_trait]
impl<'a> BuildBackend for Pibs<'a> {
    async fn process_task(&self, task: Task) -> Result<TaskSuccess, TaskError> {
        if task.multi_file() {
            let cmake_lists_path = task.src.join("CMakeLists.txt");
            if cmake_lists_path.exists() {
                return self.process_cmake_task(task).await;
            }
            let python_path = task.src.join("main.py");
            if python_path.exists() {
                let out_path = task.dest.join("out.py");
                std::fs::copy(&python_path, &out_path)?;
                let mut command = crate::command::Command::new("python3");
                command.arg(&out_path);
                return Ok(TaskSuccess { command });
            }
            return Err(TaskError::FeatureNotSupported {
                feature: "multi-file sources",
            });
        }

        let incl_arg = format!("-I{}/include", self.jjs_dir.display());
        let link_arg = format!("-L{}/lib", self.jjs_dir.display());

        let dest_file = task.dest.join("bin");
        tokio::process::Command::new("g++")
            .arg("-std=c++17")
            .arg(incl_arg)
            .arg(link_arg)
            .arg("-DPPC=1")
            .arg(task.src)
            .arg("-o")
            .arg(&dest_file)
            .arg("-ljtl")
            .arg("-lpthread")
            .arg("-ldl")
            .run()
            .await?;

        let command = crate::command::Command::new(&dest_file);
        Ok(TaskSuccess { command })
    }
}
