use std::path::Path;
use thiserror::Error;

#[derive(Debug, Copy, Clone)]
pub(crate) struct Task<'a> {
    /// Directory with source files, or path to single file
    pub(crate) src: &'a Path,
    /// Directory for build artifacts
    pub(crate) dest: &'a Path,
    /// Directort for temporary data
    pub(crate) tmp: &'a Path,
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

impl<'a> Task<'a> {
    fn multi_file(&self) -> bool {
        self.src.is_dir()
    }
}

trait CommandExt {
    fn run(&mut self) -> Result<(), TaskError>;
}

impl CommandExt for std::process::Command {
    fn run(&mut self) -> Result<(), TaskError> {
        let out = self.output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(TaskError::ExitCodeNonZero(out))
        }
    }
}

pub(crate) trait BuildBackend {
    fn process_task(&self, task: Task) -> Result<TaskSuccess, TaskError>;
}

/// Ppc-integrated build system
pub(crate) struct Pibs<'a> {
    pub(crate) jjs_dir: &'a Path,
}

impl<'a> Pibs<'a> {
    fn process_cmake_task(&self, task: Task) -> Result<TaskSuccess, TaskError> {
        //    let cmake_lists = task.src.join("CMakeLists.txt");
        std::process::Command::new("cmake")
            .arg("-S")
            .arg(task.src)
            .arg("-B")
            .arg(task.tmp)
            .run()?;

        std::process::Command::new("cmake")
            .arg("--build")
            .arg(task.tmp)
            .run()?;

        let dst = task.dest.join("bin");
        std::fs::copy(task.tmp.join("Out"), &dst)?;
        let run_cmd = crate::command::Command::new(dst);
        Ok(TaskSuccess { command: run_cmd })
    }
}
impl<'a> BuildBackend for Pibs<'a> {
    fn process_task(&self, task: Task) -> Result<TaskSuccess, TaskError> {
        if task.multi_file() {
            let cmake_lists_path = task.src.join("CMakeLists.txt");
            if cmake_lists_path.exists() {
                return self.process_cmake_task(task);
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
        std::process::Command::new("g++")
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
            .run()?;

        let command = crate::command::Command::new(&dest_file);
        Ok(TaskSuccess { command })
    }
}
