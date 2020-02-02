use crate::controller::{ControllerDriver, InvocationFinishReason};
use anyhow::Context;
use invoker_api::InvokeTask;
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

struct SillyDriverState {
    queue: VecDeque<InvokeTask>,
    messages: VecDeque<Message>,
}
pub struct SillyDriver {
    state: Arc<Mutex<SillyDriverState>>,
}

impl SillyDriver {
    pub fn new() -> SillyDriver {
        let state = SillyDriverState {
            queue: VecDeque::new(),
            messages: VecDeque::new(),
        };
        let state = Arc::new(Mutex::new(state));
        SillyDriver { state }
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

impl Default for SillyDriver {
    fn default() -> SillyDriver {
        SillyDriver::new()
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

impl ControllerDriver for SillyDriver {
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
