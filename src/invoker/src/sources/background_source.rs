use crate::controller::{InvocationFinishReason, TaskSource};
use anyhow::Context;
use invoker_api::InvokeTask;
use serde::Serialize;
use std::{collections::VecDeque, sync::Mutex};
use uuid::Uuid;

struct BackgroundSourceState {
    queue: VecDeque<InvokeTask>,
    messages: VecDeque<Message>,
}
pub struct BackgroundSource {
    state: Mutex<BackgroundSourceState>,
}

impl BackgroundSource {
    pub fn new() -> BackgroundSource {
        let state = BackgroundSourceState {
            queue: VecDeque::new(),
            messages: VecDeque::new(),
        };
        let state = Mutex::new(state);
        BackgroundSource { state }
    }

    pub fn add_task(&self, task: InvokeTask) {
        let mut st = self.state.lock().unwrap();
        st.queue.push_back(task);
    }

    pub fn pop_msg(&self) -> Option<Message> {
        let mut st = self.state.lock().unwrap();
        st.messages.pop_front()
    }
}

impl Default for BackgroundSource {
    fn default() -> BackgroundSource {
        BackgroundSource::new()
    }
}

#[derive(Serialize)]
pub enum Message {
    Finish(FinishedMessage),
    Progress(ProgressMessage),
}
#[derive(Serialize)]
pub struct FinishedMessage {
    invocation_id: Uuid,
    reason: &'static str,
}

#[derive(Serialize)]
pub struct ProgressMessage {
    invocation_id: Uuid,
    header: invoker_api::InvokeOutcomeHeader,
}

impl TaskSource for BackgroundSource {
    fn load_tasks(&self, cnt: usize) -> anyhow::Result<Vec<InvokeTask>> {
        let mut q = self.state.lock().unwrap();
        let q = &mut q.queue;
        Ok(q.drain(0..cnt.min(q.len())).collect())
    }

    fn set_finished(
        &self,
        invocation_id: Uuid,
        reason: InvocationFinishReason,
    ) -> anyhow::Result<()> {
        let reason = match reason {
            InvocationFinishReason::CompileError => "CompileError",
            InvocationFinishReason::JudgeDone => "JudgeDone",
            InvocationFinishReason::Fault => "Fault",
        };
        let msg = FinishedMessage {
            reason,
            invocation_id,
        };
        let msg = serde_json::to_string(&msg).context("serialization error")?;
        println!("{}", msg);
        Ok(())
    }

    fn add_outcome_header(
        &self,
        invocation_id: Uuid,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> anyhow::Result<()> {
        let msg = ProgressMessage {
            invocation_id,
            header,
        };
        let msg = serde_json::to_string(&msg).context("serialization error")?;
        println!("{} ", msg);
        Ok(())
    }
}
