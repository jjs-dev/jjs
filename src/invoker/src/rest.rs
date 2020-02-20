use crate::{controller::TaskSource};
use std::sync::{Arc,Mutex};
use invoker_api::{rest::HttpInvokeTask};
use slog_scope::{info};
fn transform_task(task: HttpInvokeTask) -> Task {
     info!("Processing HttpInvokeTask {}", task);
}

struct Task {
    
}

struct Tasks(Arc<Mutex<Vec<Task>>>);

impl TaskSource for Tasks {
    fn load_tasks(&self, cnt: usize) -> anyhow::Result<Vec<invoker_api::InvokeTask>> {
        let mut tasks = Vec::new();
        
        Ok(tasks)
    }
}

struct ServiceState {
    tasks: Tasks
}