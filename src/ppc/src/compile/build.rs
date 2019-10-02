use std::path::Path;

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

pub(crate) type TaskError = Box<dyn std::error::Error + 'static>;

impl<'a> Task<'a> {
    fn multi_file(&self) -> bool {
        self.src.is_dir()
    }
}

trait CommandExt {
    fn run(&mut self) -> Result<(), TaskError>;
}

#[derive(Debug)]
struct CommandRunErr(std::process::Output);

impl std::fmt::Display for CommandRunErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let status = match self.0.status.code() {
            Some(s) => s.to_string(),
            None => "<unknown exit status>".to_string(),
        };

        write!(
            f,
            "command failed with code {}: {}{}",
            status,
            String::from_utf8_lossy(&self.0.stdout),
            String::from_utf8_lossy(&self.0.stderr)
        )
    }
}

impl std::error::Error for CommandRunErr {}

impl CommandExt for std::process::Command {
    fn run(&mut self) -> Result<(), TaskError> {
        let out = self.output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(Box::new(CommandRunErr(out)))
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

impl<'a> BuildBackend for Pibs<'a> {
    fn process_task(&self, task: Task) -> Result<TaskSuccess, TaskError> {
        if task.multi_file() {
            return Err("multi-file sources not supported yet".into());
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
            .arg("-ljtlrs")
            .arg("-lpthread")
            .arg("-ldl")
            .run()?;

        let command = crate::command::Command::new(&dest_file);
        Ok(TaskSuccess { command })
    }
}
